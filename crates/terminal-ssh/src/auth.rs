use crate::{host_key::HostKeyHandler, network, ssh_error};
use russh::{
    MethodKind,
    client::{AuthResult, Handle, KeyboardInteractiveAuthResponse},
    keys::{HashAlg, PrivateKey, PrivateKeyWithHashAlg},
};
use std::{sync::Arc, time::Duration};
use terminal_core::*;

pub(crate) async fn authenticate(
    connection: &mut Handle<HostKeyHandler>,
    config: &SshConfig,
    context: &DriverContext,
    credentials: &dyn CredentialStore,
) -> Result<()> {
    tokio::time::timeout(
        Duration::from_secs(600),
        authenticate_plan(connection, config, context, credentials),
    )
    .await
    .map_err(|_| ssh_error("AUTH_TIMEOUT", "authentication", "认证总时限已到"))?
}
async fn authenticate_plan(
    connection: &mut Handle<HostKeyHandler>,
    config: &SshConfig,
    context: &DriverContext,
    credentials: &dyn CredentialStore,
) -> Result<()> {
    // 先查询服务端支持的方法，避免仅支持 PAM 的主机先弹出无效的密码表单。
    // 此请求发生在主机身份校验之后，不读取或发送任何秘密。
    let mut remaining = match network(
        "authentication",
        connection.authenticate_none(config.username.clone()),
    )
    .await?
    {
        AuthResult::Success => return Ok(()),
        AuthResult::Failure {
            remaining_methods, ..
        } => remaining_methods,
    };
    let mut rounds = 0;
    let mut submissions = 0;
    let mut interactive_started = false;
    let mut interactive_password = config.auth_plan.iter().find_map(|step| match step {
        AuthStep::Password { credential_id } => credential_id.as_deref(),
        AuthStep::PrivateKey { .. } => None,
    });
    for step in &config.auth_plan {
        let result = match step {
            AuthStep::Password { credential_id } => {
                if !remaining.contains(&MethodKind::Password) {
                    continue;
                }
                let mut outcome = None;
                for attempt in 0..3 {
                    if submissions >= 3 {
                        break;
                    }
                    let password = if attempt == 0 {
                        match credential_id {
                            Some(id) => credentials.secret(&context.owner, id).await?,
                            None => password_prompt(context, attempt).await?,
                        }
                    } else {
                        password_prompt(context, attempt).await?
                    };
                    // 已尝试密码协议后，PAM 回退不再自动重发已提交的密码。
                    interactive_password = None;
                    submissions += 1;
                    let result = network(
                        "authentication",
                        connection
                            .authenticate_password(config.username.clone(), password.as_str()),
                    )
                    .await?;
                    let stop = match &result {
                        AuthResult::Success => return Ok(()),
                        AuthResult::Failure {
                            remaining_methods,
                            partial_success,
                        } => *partial_success || !remaining_methods.contains(&MethodKind::Password),
                    };
                    outcome = Some(result);
                    if stop {
                        break;
                    }
                }
                match outcome {
                    Some(result) => result,
                    None => continue,
                }
            }
            AuthStep::PrivateKey { credential_id } => {
                if !remaining.contains(&MethodKind::PublicKey) {
                    continue;
                }
                let material = credentials
                    .private_key(&context.owner, credential_id)
                    .await?;
                let text = material.private_key;
                if text.len() > 1024 * 1024 {
                    return Err(ssh_error(
                        "LIMIT_EXCEEDED",
                        "authentication",
                        "私钥文件超过 1 MiB 限制",
                    ));
                }
                let mut key = decode(text.clone(), material.passphrase).await;
                for attempt in 0..3 {
                    if key.is_ok() {
                        break;
                    }
                    let answers = context
                        .challenges
                        .auth(
                            "私钥口令".into(),
                            if attempt == 0 {
                                "请输入所选私钥的解密口令；口令仅用于本次连接。"
                            } else {
                                "私钥解密未成功，请核对口令与文件格式。"
                            }
                            .into(),
                            vec![AuthPrompt {
                                text: "私钥口令".into(),
                                echo: false,
                            }],
                        )
                        .await?;
                    key = decode(text.clone(), answers.into_iter().next()).await;
                }
                let key = key?;
                // RSA None 会选择 SHA-1；无法确认 SHA-2 支持时明确拒绝，绝不静默回退。
                let hash = if key.algorithm().is_rsa() {
                    match network("authentication", connection.best_supported_rsa_hash()).await? {
                        Some(Some(hash @ (HashAlg::Sha256 | HashAlg::Sha512))) => Some(hash),
                        _ => {
                            return Err(ssh_error(
                                "UNSUPPORTED_OPERATION",
                                "authentication",
                                "服务器未声明支持 RSA-SHA2 签名",
                            ));
                        }
                    }
                } else {
                    None
                };
                network(
                    "authentication",
                    connection.authenticate_publickey(
                        config.username.clone(),
                        PrivateKeyWithHashAlg::new(Arc::new(key), hash),
                    ),
                )
                .await?
            }
        };
        match result {
            AuthResult::Success => return Ok(()),
            AuthResult::Failure {
                remaining_methods,
                partial_success,
            } => {
                remaining = remaining_methods;
                // 已接受首个因素时立即继续服务端挑战，不重发密码或要求用户预先勾选 OTP。
                if partial_success
                    && !interactive_started
                    && remaining.contains(&MethodKind::KeyboardInteractive)
                {
                    interactive_started = true;
                    match keyboard_interactive(
                        connection,
                        config,
                        context,
                        credentials,
                        interactive_password.take(),
                        &mut rounds,
                    )
                    .await?
                    {
                        AuthResult::Success => return Ok(()),
                        AuthResult::Failure {
                            remaining_methods, ..
                        } => remaining = remaining_methods,
                    }
                }
            }
        }
    }
    // 密码协议不可用或被拒绝时，允许服务端自身的 PAM/OTP 表单完成认证。
    // 保存密码仅用于明确的密码提示，不枚举未配置的本地私钥。
    if !interactive_started && remaining.contains(&MethodKind::KeyboardInteractive) {
        if keyboard_interactive(
            connection,
            config,
            context,
            credentials,
            interactive_password.take(),
            &mut rounds,
        )
        .await?
        .success()
        {
            return Ok(());
        }
    }
    Err(ssh_error(
        "AUTH_FAILED",
        "authentication",
        "服务器未接受配置的认证方式或仍要求不支持的额外认证因素",
    ))
}

async fn keyboard_interactive(
    connection: &mut Handle<HostKeyHandler>,
    config: &SshConfig,
    context: &DriverContext,
    credentials: &dyn CredentialStore,
    mut password_credential: Option<&str>,
    rounds: &mut usize,
) -> Result<AuthResult> {
    // 每组最多启动三次，所有重试共用八轮挑战预算，防止恶意服务端无限索取输入。
    for attempt in 0..3 {
        let mut response = network(
            "authentication",
            connection
                .authenticate_keyboard_interactive_start(config.username.clone(), None::<String>),
        )
        .await?;
        loop {
            match response {
                KeyboardInteractiveAuthResponse::Success => return Ok(AuthResult::Success),
                KeyboardInteractiveAuthResponse::Failure {
                    remaining_methods,
                    partial_success,
                } => {
                    if attempt < 2 && remaining_methods.contains(&MethodKind::KeyboardInteractive) {
                        break;
                    }
                    return Ok(AuthResult::Failure {
                        remaining_methods,
                        partial_success,
                    });
                }
                KeyboardInteractiveAuthResponse::InfoRequest {
                    name,
                    instructions,
                    prompts,
                } => {
                    *rounds += 1;
                    if *rounds > 8 {
                        return Err(ssh_error(
                            "LIMIT_EXCEEDED",
                            "authentication",
                            "认证交互超过 8 轮限制",
                        ));
                    }
                    let prompts: Vec<AuthPrompt> = prompts
                        .into_iter()
                        .map(|prompt| AuthPrompt {
                            text: prompt.prompt,
                            echo: prompt.echo,
                        })
                        .collect();
                    validate_auth_challenge(&name, &instructions, &prompts)?;
                    let answers = if prompts.is_empty()
                        && name.trim().is_empty()
                        && instructions.trim().is_empty()
                    {
                        // PAM 可在最终成功前发出空轮次，直接回零个响应，不创建空白弹窗。
                        Vec::new()
                    } else if is_password_prompt(&prompts)
                        && let Some(id) = password_credential.take()
                    {
                        // 每次连接最多自动填入一次；重复提示或失败重试必须由用户输入。
                        vec![credentials.secret(&context.owner, id).await?]
                    } else {
                        context.challenges.auth(name, instructions, prompts).await?
                    };
                    // russh 要求拥有字符串；短期副本不记录或持久化，OTP 仅用于本次响应。
                    response = network(
                        "authentication",
                        connection.authenticate_keyboard_interactive_respond(
                            answers
                                .iter()
                                .map(|answer| answer.as_str().to_owned())
                                .collect(),
                        ),
                    )
                    .await?;
                }
            }
        }
    }
    Err(ssh_error("AUTH_FAILED", "authentication", "交互认证未完成"))
}
fn is_password_prompt(prompts: &[AuthPrompt]) -> bool {
    let [prompt] = prompts else { return false };
    if prompt.echo {
        return false;
    }
    let label = prompt.text.trim().trim_end_matches([':', '：']).trim();
    label.eq_ignore_ascii_case("password") || label == "密码"
}

async fn password_prompt(context: &DriverContext, attempt: usize) -> Result<Zeroizing<String>> {
    let answers = context
        .challenges
        .auth(
            "SSH 密码".into(),
            if attempt == 0 {
                "密码仅用于本次连接，不会自动保存。"
            } else {
                "上次密码未被接受，请重新输入。"
            }
            .into(),
            vec![AuthPrompt {
                text: "密码".into(),
                echo: false,
            }],
        )
        .await?;
    answers
        .into_iter()
        .next()
        .ok_or_else(|| ssh_error("AUTH_FAILED", "authentication", "缺少密码响应"))
}
async fn decode(
    text: Zeroizing<String>,
    passphrase: Option<Zeroizing<String>>,
) -> Result<PrivateKey> {
    tokio::task::spawn_blocking(move || {
        russh::keys::decode_secret_key(
            text.as_str(),
            passphrase.as_ref().map(|value| value.as_str()),
        )
        .map_err(|_| {
            ssh_error(
                "AUTH_FAILED",
                "authentication",
                "私钥格式不受支持或解密口令错误",
            )
        })
    })
    .await
    .map_err(|_| ssh_error("AUTH_FAILED", "authentication", "私钥解析工作器不可用"))?
}
