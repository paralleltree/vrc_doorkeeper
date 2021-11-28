pub mod log;

#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    OnPlayerJoined { user_name: String },
    OnPlayerLeft { user_name: String },
    UserAuthenticated { user_name: String },
}
