use base64;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref ON_PLAYER_JOINED_ROOM_ICON: String =
        base64::encode(include_bytes!("assets/joined.png"));
    pub static ref ON_PLAYER_LEFT_ROOM_ICON: String =
        base64::encode(include_bytes!("assets/left.png"));
}
