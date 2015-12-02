pub mod error;
pub mod auth;

#[tl_complex(id)]
pub enum InputPeer {
    #[tl_id("7f3b18ea")] Empty,
    #[tl_id("7da07ec9")] SelfPeer, // Self is a keyword
    #[tl_id("1023dbe8")] Contact(i32),
    #[tl_id("9b447325")] Foreign(i32, i64),
    #[tl_id("179be863")] Chat(i32),
}
