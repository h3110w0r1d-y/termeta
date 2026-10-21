pub mod backup;
mod master_key;
pub use master_key::{PasswordChangeResult, VaultStatus};
mod private_key;
mod vault;

pub use vault::{CredentialVault, SecurityError};
