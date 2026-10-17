pub mod backup;
mod master_key;
pub use master_key::{PasswordChangeResult, VaultStatus};
mod private_key;
mod vault;

pub use private_key::validate_key_file;
pub use vault::{CredentialVault, SecurityError};
