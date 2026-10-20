//! 短期 proof 与窗口代次只留在 Rust；独立门闩不占用会话或全局设置锁。
use super::{dto::Provider, failure};
use std::{
    collections::HashMap,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime},
};
use terminal_core::Result;
use zeroize::Zeroizing;

/// 服务端绝对期限转换为单调时钟；后续等待不受系统时钟调整影响，上限仍为十分钟。
pub fn authorization_deadline(expires_at: &str) -> Result<Instant> {
    let deadline = || {
        if expires_at.len() > 64 {
            return None;
        }
        let expires_at = chrono::DateTime::parse_from_rfc3339(expires_at).ok()?;
        let now: chrono::DateTime<chrono::Utc> = SystemTime::now().into();
        let remaining = expires_at.signed_duration_since(now).to_std().ok()?;
        if remaining.is_zero() {
            return None;
        }
        Instant::now().checked_add(remaining.min(Duration::from_secs(600)))
    };
    deadline().ok_or_else(|| failure("ACCOUNT_EXPIRED", "浏览器登录已过期或期限无效，请重新发起"))
}

#[derive(Clone)]
pub struct Stamp {
    epoch: u64,
    owner: String,
    owner_epoch: u64,
}
#[derive(Clone)]
pub struct Flow {
    pub provider: Provider,
    pub verifier: Zeroizing<String>,
    pub confirmation_code: Option<Zeroizing<String>>,
    pub stamp: Stamp,
    pub deadline: Instant,
    pub last_poll: Option<Instant>,
}
#[derive(Default)]
pub struct AccountRuntime {
    pub pending: Mutex<Option<super::storage::SavedAccount>>,
    pub operation: tokio::sync::Mutex<()>,
    pub commit: tokio::sync::Mutex<()>,
    epoch: AtomicU64,
    owners: Mutex<HashMap<String, u64>>,
    flows: Mutex<HashMap<String, Flow>>,
}
impl AccountRuntime {
    pub fn stamp(&self, owner: &str) -> Stamp {
        Stamp {
            epoch: self.epoch.load(Ordering::Acquire),
            owner: owner.into(),
            owner_epoch: *self.owners.lock().unwrap().get(owner).unwrap_or(&0),
        }
    }
    pub fn valid(&self, stamp: &Stamp) -> bool {
        stamp.epoch == self.epoch.load(Ordering::Acquire)
            && stamp.owner_epoch == *self.owners.lock().unwrap().get(&stamp.owner).unwrap_or(&0)
    }
    pub fn check(&self, stamp: &Stamp) -> Result<()> {
        if self.valid(stamp) {
            Ok(())
        } else {
            Err(failure("ACCOUNT_CANCELLED", "账号操作已取消，请重新操作"))
        }
    }
    pub fn invalidate(&self) {
        self.epoch.fetch_add(1, Ordering::AcqRel);
        self.flows.lock().unwrap().clear();
    }
    pub fn clear_owner(&self, owner: &str) {
        *self.owners.lock().unwrap().entry(owner.into()).or_default() += 1;
        self.flows
            .lock()
            .unwrap()
            .retain(|_, flow| flow.stamp.owner != owner);
    }
    pub fn insert(&self, id: String, flow: Flow) -> Result<()> {
        let mut flows = self.flows.lock().unwrap();
        self.check(&flow.stamp)?;
        if Instant::now() >= flow.deadline {
            return Err(failure("ACCOUNT_EXPIRED", "浏览器登录已过期，请重新发起"));
        }
        flows.retain(|_, flow| Instant::now() < flow.deadline);
        if flows.len() >= 2
            || flows
                .values()
                .any(|other| other.stamp.owner == flow.stamp.owner)
        {
            return Err(failure("ACCOUNT_BUSY", "请先完成或取消当前浏览器登录"));
        }
        flows.insert(id, flow);
        Ok(())
    }
    pub fn flow(&self, owner: &str, id: &str, polling: bool) -> Result<Flow> {
        let mut flows = self.flows.lock().unwrap();
        let flow = flows
            .get_mut(id)
            .filter(|flow| flow.stamp.owner == owner)
            .ok_or_else(|| failure("ACCOUNT_CANCELLED", "浏览器登录已取消或过期"))?;
        self.check(&flow.stamp)?;
        if Instant::now() >= flow.deadline {
            flows.remove(id);
            return Err(failure("ACCOUNT_EXPIRED", "浏览器登录已过期，请重新发起"));
        }
        if polling {
            if flow
                .last_poll
                .is_some_and(|last| last.elapsed() < Duration::from_secs(3))
            {
                return Err(failure("ACCOUNT_RATE_LIMITED", "请稍候再检查登录结果"));
            }
            flow.last_poll = Some(Instant::now());
        }
        Ok(flow.clone())
    }
    /// 确认码只写入所属窗口的现存流程，不触发网络或提前改变轮询间隔。
    pub fn confirm(&self, owner: &str, id: &str, code: Zeroizing<String>) -> Result<()> {
        self.confirm_matching(owner, id, None, code)
    }
    /// 系统回跳从本机 flow 取得 owner，不接受链接指定窗口。
    pub fn callback_owner(&self, id: &str, provider: Provider) -> Result<String> {
        let flows = self.flows.lock().unwrap();
        let flow = flows
            .get(id)
            .filter(|flow| flow.provider == provider)
            .ok_or_else(|| failure("ACCOUNT_CANCELLED", "浏览器登录已取消或过期"))?;
        self.check(&flow.stamp)?;
        if Instant::now() >= flow.deadline {
            return Err(failure("ACCOUNT_EXPIRED", "浏览器登录已过期，请重新发起"));
        }
        Ok(flow.stamp.owner.clone())
    }
    pub fn confirm_callback(
        &self,
        owner: &str,
        id: &str,
        provider: Provider,
        code: Zeroizing<String>,
    ) -> Result<()> {
        self.confirm_matching(owner, id, Some(provider), code)
    }
    fn confirm_matching(
        &self,
        owner: &str,
        id: &str,
        provider: Option<Provider>,
        code: Zeroizing<String>,
    ) -> Result<()> {
        if code.len() != 32
            || !code
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(failure(
                "ACCOUNT_INVALID",
                "请输入浏览器显示的 32 字符确认码",
            ));
        }
        let mut flows = self.flows.lock().unwrap();
        let flow = flows
            .get_mut(id)
            .filter(|flow| {
                flow.stamp.owner == owner && provider.is_none_or(|value| flow.provider == value)
            })
            .ok_or_else(|| failure("ACCOUNT_CANCELLED", "浏览器登录已取消或过期"))?;
        self.check(&flow.stamp)?;
        if Instant::now() >= flow.deadline {
            flows.remove(id);
            return Err(failure("ACCOUNT_EXPIRED", "浏览器登录已过期，请重新发起"));
        }
        if let Some(existing) = &flow.confirmation_code {
            if existing.as_str() == code.as_str() {
                return Ok(());
            }
            return Err(failure(
                "ACCOUNT_INVALID",
                "确认码已提交，请取消后重新发起授权",
            ));
        }
        flow.confirmation_code = Some(code);
        Ok(())
    }
    pub fn remove(&self, owner: &str, id: &str) -> Option<Flow> {
        let mut flows = self.flows.lock().unwrap();
        if flows.get(id).is_some_and(|flow| flow.stamp.owner == owner) {
            flows.remove(id)
        } else {
            None
        }
    }
    pub fn contains(&self, owner: &str, id: &str) -> bool {
        self.flows
            .lock()
            .unwrap()
            .get(id)
            .is_some_and(|flow| flow.stamp.owner == owner)
    }
    pub fn cleanup(&self) {
        self.flows
            .lock()
            .unwrap()
            .retain(|_, flow| Instant::now() < flow.deadline);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending(runtime: &AccountRuntime, owner: &str) -> Flow {
        Flow {
            provider: Provider::Github,
            verifier: Zeroizing::new("test-only-verifier".into()),
            confirmation_code: None,
            stamp: runtime.stamp(owner),
            deadline: Instant::now() + Duration::from_secs(300),
            last_poll: None,
        }
    }

    #[test]
    fn server_expiry_is_parsed_bounded_and_kept_as_a_monotonic_deadline() {
        assert!(authorization_deadline("invalid").is_err());
        assert!(authorization_deadline("2000-01-01T00:00:00Z").is_err());
        let now: chrono::DateTime<chrono::Utc> = SystemTime::now().into();
        let short = (now + chrono::TimeDelta::seconds(60)).to_rfc3339();
        let remaining = authorization_deadline(&short)
            .unwrap()
            .duration_since(Instant::now());
        assert!(remaining > Duration::from_secs(55));
        assert!(remaining <= Duration::from_secs(60));
        let long = (now + chrono::TimeDelta::seconds(3600)).to_rfc3339();
        assert!(
            authorization_deadline(&long)
                .unwrap()
                .duration_since(Instant::now())
                <= Duration::from_secs(600)
        );
    }

    #[test]
    fn native_return_requires_matching_provider_owner_and_live_generation() {
        let runtime = AccountRuntime::default();
        runtime
            .insert("flow".into(), pending(&runtime, "settings"))
            .unwrap();
        assert!(runtime.callback_owner("unknown", Provider::Github).is_err());
        assert!(runtime.callback_owner("flow", Provider::Google).is_err());
        assert_eq!(
            runtime.callback_owner("flow", Provider::Github).unwrap(),
            "settings"
        );
        assert!(
            runtime
                .confirm_callback(
                    "settings",
                    "flow",
                    Provider::Google,
                    Zeroizing::new("a".repeat(32))
                )
                .is_err()
        );
        assert!(
            runtime
                .confirm_callback(
                    "main",
                    "flow",
                    Provider::Github,
                    Zeroizing::new("a".repeat(32))
                )
                .is_err()
        );
        for _ in 0..2 {
            runtime
                .confirm_callback(
                    "settings",
                    "flow",
                    Provider::Github,
                    Zeroizing::new("a".repeat(32)),
                )
                .unwrap();
        }
        assert!(
            runtime
                .confirm_callback(
                    "settings",
                    "flow",
                    Provider::Github,
                    Zeroizing::new("b".repeat(32))
                )
                .is_err()
        );
        runtime.invalidate();
        assert!(runtime.callback_owner("flow", Provider::Github).is_err());
    }

    #[test]
    fn native_return_never_revives_expired_flow_or_restarted_app() {
        let runtime = AccountRuntime::default();
        assert!(runtime.callback_owner("flow", Provider::Github).is_err());
        let mut flow = pending(&runtime, "settings");
        flow.deadline = Instant::now() - Duration::from_secs(1);
        // insert 会清理旧流程，先登记后模拟到期。
        runtime
            .insert("flow".into(), pending(&runtime, "settings"))
            .unwrap();
        runtime.flows.lock().unwrap().insert("flow".into(), flow);
        assert!(runtime.callback_owner("flow", Provider::Github).is_err());
        assert!(
            runtime
                .confirm_callback(
                    "settings",
                    "flow",
                    Provider::Github,
                    Zeroizing::new("a".repeat(32))
                )
                .is_err()
        );
        assert!(!runtime.contains("settings", "flow"));
    }

    #[test]
    fn confirmation_is_owner_bound_and_applies_to_next_poll_snapshot() {
        let runtime = AccountRuntime::default();
        runtime
            .insert("flow".into(), pending(&runtime, "settings"))
            .unwrap();
        let in_flight = runtime.flow("settings", "flow", true).unwrap();
        assert!(runtime.flow("main", "flow", false).is_err());
        assert!(
            runtime
                .confirm("main", "flow", Zeroizing::new("a".repeat(32)))
                .is_err()
        );
        runtime
            .confirm("settings", "flow", Zeroizing::new("a".repeat(32)))
            .unwrap();
        assert!(in_flight.confirmation_code.is_none());
        assert_eq!(
            runtime
                .flow("settings", "flow", false)
                .unwrap()
                .confirmation_code
                .unwrap()
                .as_str(),
            "a".repeat(32)
        );
        // 本机录码不能绕过既有轮询间隔，也不改变已发送请求的快照。
        assert!(runtime.flow("settings", "flow", true).is_err());
    }

    #[test]
    fn cancelled_or_destroyed_owner_cannot_reuse_proof() {
        let runtime = AccountRuntime::default();
        runtime
            .insert("cancelled".into(), pending(&runtime, "settings"))
            .unwrap();
        assert!(runtime.remove("main", "cancelled").is_none());
        assert!(runtime.remove("settings", "cancelled").is_some());
        assert!(
            runtime
                .confirm("settings", "cancelled", Zeroizing::new("a".repeat(32)))
                .is_err()
        );
        assert!(runtime.flow("settings", "cancelled", false).is_err());

        let old_flow = pending(&runtime, "settings");
        runtime
            .insert("destroyed".into(), old_flow.clone())
            .unwrap();
        runtime.clear_owner("settings");
        assert!(runtime.check(&old_flow.stamp).is_err());
        assert!(runtime.flow("settings", "destroyed", false).is_err());
        // 同名重开窗口不能重新登记旧窗口持有的 proof。
        assert!(runtime.insert("replayed".into(), old_flow).is_err());
    }

    #[test]
    fn logout_invalidates_all_owners_and_rejects_late_insert() {
        let runtime = AccountRuntime::default();
        let main = pending(&runtime, "main");
        let settings = pending(&runtime, "settings");
        runtime.insert("main".into(), main.clone()).unwrap();
        runtime.insert("settings".into(), settings.clone()).unwrap();
        runtime.invalidate();
        assert!(runtime.check(&main.stamp).is_err());
        assert!(runtime.check(&settings.stamp).is_err());
        assert!(
            runtime
                .confirm("main", "main", Zeroizing::new("a".repeat(32)))
                .is_err()
        );
        assert!(runtime.insert("late".into(), settings).is_err());
    }

    #[test]
    fn confirmation_encoding_expiry_and_local_poll_budget_are_enforced() {
        let runtime = AccountRuntime::default();
        runtime
            .insert("flow".into(), pending(&runtime, "settings"))
            .unwrap();
        for code in [
            "a".repeat(31),
            "a".repeat(33),
            format!("{}=", "a".repeat(31)),
            "你".repeat(32),
        ] {
            assert!(
                runtime
                    .confirm("settings", "flow", Zeroizing::new(code))
                    .is_err()
            );
        }
        runtime
            .confirm("settings", "flow", Zeroizing::new("a".repeat(32)))
            .unwrap();
        assert!(
            runtime
                .confirm("settings", "flow", Zeroizing::new("b".repeat(32)))
                .is_err()
        );
        assert!(runtime.flow("settings", "flow", true).is_ok());
        assert!(runtime.flow("settings", "flow", true).is_err());
        runtime
            .flows
            .lock()
            .unwrap()
            .get_mut("flow")
            .unwrap()
            .last_poll = Some(Instant::now() - Duration::from_secs(4));
        assert!(runtime.flow("settings", "flow", true).is_ok());
        runtime
            .flows
            .lock()
            .unwrap()
            .get_mut("flow")
            .unwrap()
            .deadline = Instant::now() - Duration::from_secs(1);
        assert!(
            runtime
                .confirm("settings", "flow", Zeroizing::new("a".repeat(32)))
                .is_err()
        );
        assert!(!runtime.contains("settings", "flow"));
    }
}
