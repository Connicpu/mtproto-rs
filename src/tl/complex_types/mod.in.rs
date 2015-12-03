use tl::Vector;

pub mod auth;
pub mod storage;
pub mod contacts;
pub mod messages;
pub mod updates;
pub mod photos;
pub mod upload;
pub mod help;

#[derive(TLType)]
#[tl_id(_c4b9f9bb)]
pub struct Error {
    pub code: i32,
    pub text: String,
}

#[derive(TLType)]
pub enum InputPeer {
    #[tl_id(_7f3b18ea)] Empty,
    #[tl_id(_7da07ec9)] SelfPeer,
    #[tl_id(_1023dbe8)] Contact { user_id: i32 },
    #[tl_id(_9b447325)] Foreign { user_id: i32, access_hash: i64 },
    #[tl_id(_179be863)] Chat { chat_id: i32 },
}

#[derive(TLType)]
pub enum InputUser {
    #[tl_id(_b98886cf)] Empty,
    #[tl_id(_f7c1b13f)] SelfUser,
    #[tl_id(_86e94f65)] Contact { user_id: i32 },
    #[tl_id(_655e74ff)] Foreign { user_id: i32, access_hash: i64 },
}

#[derive(TLType)]
#[tl_id(_f392b7f4)]
pub struct InputContact {
    pub client_id: i64,
    pub phone: String,
    pub first_name: String,
    pub last_name: String,
}

#[derive(TLType)]
pub enum InputFile {
    #[tl_id(_f52ff27f)] Normal { id: i64, parts: i32, name: String, md5_checksum: String },
    #[tl_id(_fa4f0bb5)] Big { id: i64, parts: i32, name: String },
}

#[derive(TLType)]
pub enum InputMedia {
    #[tl_id(_9664f57f)] Empty,
    #[tl_id(_2dc53a7d)] UploadedPhoto {
        file: InputFile,
    },
    #[tl_id(_8f2ab2ec)] Photo {
        id: InputPhoto,
    },
    #[tl_id(_f9c44144)] GeoPoint {
        geo_point: InputGeoPoint,
    },
    #[tl_id(_a6e45987)] Contact {
        phone_number: String,
        first_name: String,
        last_name: String,
    },
    #[tl_id(_133ad6f6)] UploadedVideo {
        file: InputFile,
        duration: i32,
        w: i32, h: i32,
        mime_type: String,
    },
    #[tl_id(_9912dabf)] UploadedThumbVideo {
        file: InputFile,
        thumb: InputFile,
        duration: i32,
        w: i32, h: i32,
        mime_type: String,
    },
    #[tl_id(_7f023ae6)] Video {
        id: InputVideo,
    },
    #[tl_id(_4e498cab)] UploadedAudio {
        file: InputFile,
        duration: i32,
        mime_type: String,
    },
    #[tl_id(_89938781)] Audio {
        id: InputAudio,
    },
    #[tl_id(_ffe76b78)] UploadedDocument {
        file: InputFile,
        mime_type: String,
        attributes: Vector<DocumentAttribute>,
    },
    #[tl_id(_41481486)] UploadedThumbDocument {
        file: InputFile,
        thumb: InputFile,
        mime_type: String,
        attributes: Vector<DocumentAttribute>,
    },
    #[tl_id(_d184e841)] Document {
        id: InputDocument,
    },
}

#[derive(TLType)]
pub enum InputChatPhoto {
    #[tl_id(_1ca48f57)] Empty,
    #[tl_id(_94254732)] UploadedPhoto {
        file: InputFile,
        crop: InputPhotoCrop,
    },
    #[tl_id(_b2e1bf08)] Photo {
        id: InputPhoto,
        crop: InputPhotoCrop,
    },
}

#[derive(TLType)]
pub enum InputGeoPoint {
    #[tl_id(_e4c123d6)] Empty,
    #[tl_id(_f3b7acc9)] Point { lat: f64, long: f64 },
}

#[derive(TLType)]
pub enum InputPhoto {
    #[tl_id(_1cd7bf0d)] Empty,
    #[tl_id(_fb95c6c4)] Photo { id: i64, access_hash: i64 },
}

#[derive(TLType)]
pub enum InputVideo {
    #[tl_id(_5508ec75)] Empty,
    #[tl_id(_ee579652)] Video { id: i64, access_hash: i64 },
}

#[derive(TLType)]
pub enum InputFileLocation {
    #[tl_id(_14637196)] File { volume_id: i64, local_id: i32, secret: i64 },
    #[tl_id(_3d0364ec)] Video { id: i64, access_hash: i64 },
    #[tl_id(_f5235d55)] Encrypted { id: i64, access_hash: i64 },
    #[tl_id(_74dc404d)] Audio { id: i64, access_hash: i64 },
    #[tl_id(_4e45abe9)] Document { id: i64, access_hash: i64 },
}

#[derive(TLType)]
pub enum InputPhotoCrop {
    #[tl_id(_ade6b004)] Auto,
    #[tl_id(_d9915325)] Crop { left: f64, top: f64, width: f64 },
}

#[derive(TLType)]
#[tl_id(_770656a8)]
pub struct InputAppEvent {
    pub time: f64,
    pub event_type: String,
    pub peer: i64,
    pub data: String,
}

#[derive(TLType)]
pub enum Peer {
    #[tl_id(_9db1bc6d)] User { user_id: i32 },
    #[tl_id(_bad0e5bb)] Char { chat_id: i32 },
}

#[derive(TLType)]
pub enum FileLocation {
    #[tl_id(_7c596b46)] Unavailable {
        volume_id: i64,
        local_id: i32,
        secret: i64,
    },
    #[tl_id(_53d69076)] Location {
        dc_id: i32,
        volume_id: i64,
        local_id: i32,
        secret: i64,
    },
}

#[derive(TLType)]
pub enum User {
    #[tl_id(_200250ba)] Empty {
        id: i32
    },
    #[tl_id(_7007b451)] SelfUser {
        id: i32,
        first_name: String,
        last_name: String,
        username: String,
        phone: String,
        photo: UserProfilePhoto,
        status: UserStatus,
        inactive: bool,
    },
    #[tl_id(_cab35e18)] Contact {
        id: i32,
        first_name: String,
        last_name: String,
        username: String,
        access_hash: i64,
        phone: String,
        photo: UserProfilePhoto,
        status: UserStatus,
    },
    #[tl_id(_d9ccc4ef)] Request {
        id: i32,
        first_name: String,
        last_name: String,
        username: String,
        access_hash: i64,
        phone: String,
        photo: UserProfilePhoto,
        status: UserStatus,
    },
    #[tl_id(_75cf7a8)] Foreign {
        id: i32,
        first_name: String,
        last_name: String,
        username: String,
        access_hash: i64,
        photo: UserProfilePhoto,
        status: UserStatus,
    },
    #[tl_id(_d6016d7a)] Deleted {
        id: i32,
        first_name: String,
        last_name: String,
        username: String,
    }
}

#[derive(TLType)]
pub enum UserProfilePhoto {
    #[tl_id(_4f11bae1)] Empty,
    #[tl_id(_d559d8c8)] Photo {
        photo_id: i64,
        photo_small: FileLocation,
        photo_big: FileLocation,
    }
}

#[derive(TLType)]
pub enum UserStatus {
    #[tl_id(_09d05049)] Empty,
    #[tl_id(_edb93949)] Online { expires: i32 },
    #[tl_id(_008c703f)] Offline { was_online: i32 },
    #[tl_id(_e26f42f1)] Recently,
    #[tl_id(_07bf09fc)] LastWeek,
    #[tl_id(_77ebc742)] LastMonth,
}

#[derive(TLType)]
pub enum Chat {
    #[tl_id(_9ba2d800)] Empty {
        id: i32,
    },
    #[tl_id(_6e9c9bc7)] Chat {
        id: i32,
        title: String,
        photo: ChatPhoto,
        participants_count: i32,
        date: i32,
        left: bool,
        version: i32,
    },
    #[tl_id(_fb0ccc41)] Forbidden {
        id: i32,
        title: String,
        date: i32,
    },
}

#[derive(TLType)]
#[tl_id(_630e61be)]
pub struct ChatFull {
    pub id: i32,
    pub participants: ChatParticipants,
    pub chat_photo: Photo,
    pub notify_settings: PeerNotifySettings,
}

#[derive(TLType)]
#[tl_id(_c8d7493e)]
pub struct ChatParticipant {
    pub user_id: i32,
    pub inviter_id: i32,
    pub date: i32,
}

#[derive(TLType)]
pub enum ChatParticipants {
    #[tl_id(_fd2bb8a)] Forbidden {
        chat_id: i32,
    },
    #[tl_id(_7841b415)] Participants {
        chat_id: i32,
        admin_id: i32,
        participants: Vector<ChatParticipant>,
        version: i32,
    }
}

#[derive(TLType)]
pub enum ChatPhoto {
    #[tl_id(_37c1011c)] Empty,
    #[tl_id(_6153276a)] Photo {
        photo_small: FileLocation,
        photo_big: FileLocation,
    },
}

#[derive(TLType)]
pub enum Message {
    #[tl_id(_83e5de54)] Empty {
        id: i32,
    },
    #[tl_id(_567699b3)] Message {
        flags: i32,
        id: i32,
        from_id: i32,
        to_id: Peer,
        date: i32,
        message: String,
        media: MessageMedia,
    },
    #[tl_id(_a367e716)] Forwarded {
        flags: i32,
        id: i32,
        fwd_from_id: i32,
        fwd_date: i32,
        from_id: i32,
        to_id: Peer,
        date: i32,
        message: String,
        media: MessageMedia,
    },
    #[tl_id(_1d86f70e)] Service {
        flags: i32,
        id: i32,
        from_id: i32,
        to_id: Peer,
        date: i32,
        action: MessageAction,
    },
}

#[derive(TLType)]
pub enum MessageMedia {
    #[tl_id(_3ded6320)] Empty,
    #[tl_id(_c8c45a2a)] Photo(Photo),
    #[tl_id(_a2d24290)] Video(Video),
    #[tl_id(_56e0d474)] Geo(GeoPoint),
    #[tl_id(_5e7d2f39)] Contact {
        phone_number: String,
        first_name: String,
        last_name: String,
        user_id: i32,
    },
    #[tl_id(_29632a36)] Unsupported {
        bytes: Vec<u8>,
    },
    #[tl_id(_2fda2204)] Document(Document),
    #[tl_id(_c6b68300)] Audio(Audio),
}

#[derive(TLType)]
pub enum MessageAction {
    #[tl_id(_b6aef7b0)] Empty,
    #[tl_id(_a6638b9a)] ChatCreate {
        title: String,
        users: Vector<i32>,
    },
    #[tl_id(_b5a1ce5a)] ChatEditTitle {
        title: String,
    },
    #[tl_id(_7fcb13a8)] ChatEditPhoto {
        photo: Photo,
    },
    #[tl_id(_95e3fbef)] ChatDeletePhoto,
    #[tl_id(_5e3cfc4b)] ChatAddUser {
        user_id: i32,
    },
    #[tl_id(_b2ae9b0c)] ChatDeleteUser {
        user_id: i32,
    },
}

#[derive(TLType)]
#[tl_id(_ab3a99ac)]
pub struct Dialog {
    pub peer: Peer,
    pub top_message: i32,
    pub unrea_count: i32,
    pub notify_settings: PeerNotifySettings,
}

#[derive(TLType)]
pub enum Photo {
    #[tl_id(_2331b22d)] Empty {
        id: i64
    },
    #[tl_id(_22b56751)] Photo {
        id: i64,
        access_hash: i64,
        user_id: i32,
        date: i32,
        caption: String,
        geo: GeoPoint,
        sizes: Vector<PhotoSize>,
    },
}

#[derive(TLType)]
pub enum PhotoSize {
    #[tl_id(_e17e23c)] Empty {
        size_type: String,
    },
    #[tl_id(_77bfb61b)] Size {
        size_type: String,
        location: FileLocation,
        w: i32, h: i32,
        size: i32,
    },
    #[tl_id(_e9a734fa)] Cached {
        size_type: String,
        location: FileLocation,
        w: i32, h: i32,
        bytes: Vec<u8>,
    },
}

#[derive(TLType)]
pub enum Video {
    #[tl_id(_c10658a8)] Empty {
        id: i64,
    },
    #[tl_id(_388fa391)] Video {
        id: i64,
        access_hash: i64,
        user_id: i32,
        date: i32,
        caption: String,
        duration: i32,
        mime_type: String,
        size: i32,
        thumb: PhotoSize,
        dc_id: i32,
        w: i32, h: i32,
    }
}

#[derive(TLType)]
pub enum GeoPoint {
    #[tl_id(_1117dd5f)] Empty,
    #[tl_id(_2049d70c)] Point {
        long: f64,
        lat: f64,
    }
}

#[derive(TLType)]
pub enum InputNotifyPeer {
    #[tl_id(_b8bc5b0c)] Peer(Peer),
    #[tl_id(_193b4417)] Users,
    #[tl_id(_4a95e84e)] Chats,
    #[tl_id(_a429b886)] All,
}

#[derive(TLType)]
pub enum InputPeerNotifyEvents {
    #[tl_id(_f03064d8)] Empty,
    #[tl_id(_e86a2c74)] All,
}

#[derive(TLType)]
#[tl_id(_46a2ce98)]
pub struct InputPeerNotifySettings {
    pub mute_until: i32,
    pub sound: String,
    pub show_previews: bool,
    pub events_mask: i32,
}

#[derive(TLType)]
pub enum PeerNotifyEvents {
    #[tl_id(_add53cb3)] Empty,
    #[tl_id(_6d1ded88)] All,
}

#[derive(TLType)]
pub enum PeerNotifySettings {
    #[tl_id(_70a68512)] Empty,
    #[tl_id(_8d5e11ee)] Settings {
        mute_until: i32,
        sound: String,
        show_previews: bool,
        events_mask: i32,
    },
}

#[derive(TLType)]
pub enum WallPaper {
    #[tl_id(_ccb03657)] Photo {
        id: i32,
        title: String,
        sizes: Vector<PhotoSize>,
        color: i32,
    },
    #[tl_id(_63117f24)] Solid {
        id: i32,
        title: String,
        bg_color: i32,
        color: i32,
    },
}

#[derive(TLType)]
pub enum ReportReason {
    #[tl_id(_58dbcab8)] Spam,
    #[tl_id(_1e22c78d)] Violence,
    #[tl_id(_2e59d922)] Pornography,
    #[tl_id(_e1746d0a)] Other(String),
}

#[derive(TLType)]
#[tl_id(_771095da)]
pub struct UserFull {
    pub user: User,
    pub link: contacts::Link,
    pub profile_photo: Photo,
    pub notify_settings: PeerNotifySettings,
    pub blocked: bool,
    pub real_first_name: String,
    pub real_last_name: String,
}

#[derive(TLType)]
#[tl_id(_f911c994)]
pub struct Contact {
    pub user_id: i32,
    pub mutual: bool,
}

#[derive(TLType)]
#[tl_id(_d0028438)]
pub struct ImportedContact {
    pub user_id: i32,
    pub client_id: i64,
}

#[derive(TLType)]
#[tl_id(_561bc879)]
pub struct ContactBlocked {
    pub user_id: i32,
    pub date: i32,
}

#[derive(TLType)]
#[tl_id(_3de191a1)]
pub struct ContactSuggested {
    pub user_id: i32,
    pub mutual_contacts: i32,
}

#[derive(TLType)]
#[tl_id(_d3680c61)]
pub struct ContactStatus {
    pub user_id: i32,
    pub status: UserStatus,
}

// #[tl_id(_)] 
