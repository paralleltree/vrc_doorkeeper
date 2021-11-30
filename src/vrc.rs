pub mod log;

#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    OnJoinedRoom,
    OnPlayerJoined { user_name: String },
    OnLeftRoom,
    OnPlayerLeft { user_name: String },
    UserAuthenticated { user_name: String },
}
