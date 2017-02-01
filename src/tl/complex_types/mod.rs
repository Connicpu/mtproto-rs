#![allow(non_camel_case_types)]

use tl::Vector;

pub mod auth;
pub mod storage;
pub mod contacts;
pub mod messages;
pub mod updates;
pub mod photos;
pub mod upload;
pub mod help;
pub mod account;

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

#[derive(TLType)]
pub enum MessagesFilter {
    #[tl_id(_57e2f66c)] Empty,
    #[tl_id(_9609a51c)] Photos,
    #[tl_id(_9fc00e65)] Video,
    #[tl_id(_56e9f0e4)] PhotoVideo,
    #[tl_id(_d95e73bb)] PhotoVideoDocuments,
    #[tl_id(_9eddf188)] Document,
    #[tl_id(_cfc87522)] Audio,
    #[tl_id(_5afbf764)] AudioDocuments,
    #[tl_id(_7ef0dd87)] Url,
}

#[derive(TLType)]
pub enum Update {
    #[tl_id(_13abdb3)] NewMessage {
        message: Message,
        pts: i32
    },
    #[tl_id(_4e90bfd6)] MessageId {
        id: i32,
        random_id: i64,
    },
    #[tl_id(_c6649e31)] ReadMessages {
        messages: Vector<i32>,
        pts: i32,
    },
    #[tl_id(_a92bfe26)] DeleteMessages {
        messages: Vector<i32>,
        pts: i32,
    },
    #[tl_id(_5c486927)] UserTypings {
        user_id: i32,
        action: SendMessageAction,
    },
    #[tl_id(_9a65ea1f)] ChatUserTypings {
        chat_id: i32,
        user_id: i32,
        action: SendMessageAction,
    },
    #[tl_id(_7761198)] ChatParticipants {
        participants: ChatParticipants,
    },
    #[tl_id(_1bfbd823)] UserStatus {
        user_id: i32,
        status: UserStatus,
    },
    #[tl_id(_a7332b73)] UserName {
        user_id: i32,
        first_name: String,
        last_name: String,
        username: String,
    },
    #[tl_id(_95313b0c)] UserPhoto {
        user_id: i32,
        date: i32,
        photo: UserProfilePhoto,
        previous: bool,
    },
    #[tl_id(_2575bbb9)] ContactRegistered {
        user_id: i32,
        date: i32,
    },
    #[tl_id(_51a48a9a)] ContactLink {
        user_id: i32,
        my_link: contacts::MyLink,
        foreign_link: contacts::ForeignLink,
    },
    #[tl_id(_8f06529a)] NewAuthorization {
        auth_key_id: i64,
        date: i32,
        device: String,
        location: String,
    },
    #[tl_id(_12bcbd9a)] NewEncryptedMessage {
        message: EncryptedMessage,
        qts: i32,
    },
    #[tl_id(_1710f156)] EncryptedChatTypings {
        chat_id: i32,
    },
    #[tl_id(_b4a2e88d)] Encryption {
        chat: EncryptedChat,
        date: i32,
    },
    #[tl_id(_38fe25b7)] EncryptedMessagesRead {
        chat_id: i32,
        max_date: i32,
        date: i32,
    },
    #[tl_id(_3a0eeb22)] ChatParticipantAdd {
        chat_id: i32,
        user_id: i32,
        inviter_id: i32,
        version: i32,
    },
    #[tl_id(_6e5f8c22)] ChatParticipantDelete {
        chat_id: i32,
        user_id: i32,
        version: i32,
    },
    #[tl_id(_8e5e9873)] DcOptions {
        dc_options: Vector<DcOption>,
    },
    #[tl_id(_80ece81a)] UserBlocked {
        user_id: i32,
        blocked: bool,
    },
    #[tl_id(_bec268ef)] NotifySettings {
        peer: NotifyPeer,
        notify_settings: PeerNotifySettings,
    },
    #[tl_id(_382dd3e4)] ServiceNotification {
        service_type: String,
        message: String,
        media: MessageMedia,
        popup: bool,
    },
    #[tl_id(_ee3b272a)] Privacy {
        key: PrivacyKey,
        rules: Vector<PrivacyRule>,
    },
    #[tl_id(_12b9417b)] UserPhone {
        user_id: i32,
        phone: String,
    },
}

#[derive(TLType)]
pub enum Updates {
    #[tl_id(_e317af7e)] TooLong,
    #[tl_id(_d3f45784)] ShortMessage {
        id: i32,
        from_id: i32,
        message: String,
        pts: i32,
        date: i32,
        seq: i32,
    },
    #[tl_id(_2b2fbd4e)] ShortChatMessage {
        id: i32,
        from_id: i32,
        chat_id: i32,
        message: String,
        pts: i32,
        date: i32,
        seq: i32,
    },
    #[tl_id(_78d4dec1)] Short {
        update: Update,
        date: i32,
    },
    #[tl_id(_725b04c3)] Combined {
        updates: Vector<Update>,
        users: Vector<User>,
        chats: Vector<Chat>,
        date: i32,
        seq_start: i32,
        seq: i32,
    },
    #[tl_id(_74ae4240)] Updates {
        updates: Vector<Update>,
        users: Vector<User>,
        chats: Vector<Chat>,
        date: i32,
        seq: i32,
    },
}

#[derive(Debug, TLType)]
#[tl_id(_2ec2a43c)]
pub struct DcOption {
    pub id: i32,
    pub hostname: String,
    pub ip_address: String,
    pub port: i32,
}

#[derive(Debug, TLType)]
#[tl_id(_7dae33e0)]
pub struct Config {
    pub date: i32,
    pub expires: i32,
    pub test_mode: bool,
    pub this_dc: i32,
    pub dc_options: Vector<DcOption>,
    pub chat_big_size: i32,
    pub chat_size_max: i32,
    pub broadcast_size_max: i32,
    pub disabled_features: Vector<DisabledFeature>,
}

#[derive(TLType)]
#[tl_id(_8e1a1775)]
pub struct NearestDc {
    pub country: String,
    pub this_dc: i32,
    pub nearest_dc: i32,
}

#[derive(TLType)]
pub enum EncryptedChat {
    #[tl_id(_ab7ec0a0)] Empty {
        id: i32,
    },
    #[tl_id(_3bf703dc)] Waiting {
        id: i32,
        access_hash: i64,
        date: i32,
        admin_id: i32,
        participant_id: i32,

    },
    #[tl_id(_c878527e)] Requested {
        id: i32,
        access_hash: i64,
        date: i32,
        admin_id: i32,
        participant_id: i32,
        g_a: Vec<u8>,
    },
    #[tl_id(_fa56ce36)] Chat {
        id: i32,
        access_hash: i64,
        date: i32,
        admin_id: i32,
        participant_id: i32,
        g_a_or_b: Vec<u8>,
        key_fingerprint: i64,
    },
    #[tl_id(_13d6dd27)] Discarded {
        id: i32,
    },
}

#[derive(TLType)]
#[tl_id(_f141b5e1)]
pub struct InputEncryptedChat {
    pub chat_id: i32,
    pub access_hash: i64,
}

#[derive(TLType)]
pub enum EncryptedFile {
    #[tl_id(_c21f497e)] Empty,
    #[tl_id(_4a70994c)] File {
        id: i64,
        access_hash: i64,
        size: i32,
        dc_id: i32,
        key_fingerprint: i32,
    },
}

#[derive(TLType)]
pub enum InputEncryptedFile {
    #[tl_id(_1837c364)] Empty,
    #[tl_id(_64bd0306)] Uploaded {
        id: i64,
        parts: i32,
        md5_checksum: String,
        key_fingerprint: i32,
    },
    #[tl_id(_5a17b5e5)] File {
        id: i64,
        access_hash: i64,
    },
    #[tl_id(_2dc173c8)] BigUploaded {
        id: i64,
        parts: i32,
        key_fingerprint: i32,
    },
}

#[derive(TLType)]
pub enum EncryptedMessage {
    #[tl_id(_ed18c118)] Message {
        random_id: i64,
        chat_id: i32,
        date: i32,
        bytes: Vec<u8>,
        file: EncryptedFile,
    },
    #[tl_id(_23734b06)] Service {
        random_id: i64,
        chat_id: i32,
        date: i32,
        bytes: Vec<u8>,
    },
}

#[derive(TLType)]
pub enum InputAudio {
    #[tl_id(_d95adc84)] Empty,
    #[tl_id(_77d440ff)] Audio {
        id: i64,
        access_hash: i64,
    },
}

#[derive(TLType)]
pub enum InputDocument {
    #[tl_id(_72f0eaae)] Empty,
    #[tl_id(_18798952)] Document {
        id: i64,
        access_hash: i64,
    },
}

#[derive(TLType)]
pub enum Audio {
    #[tl_id(_586988d8)] Empty {
        id: i64,
    },
    #[tl_id(_c7ac6496)] Audio {
        id: i64,
        access_hash: i64,
        user_id: i32,
        date: i32,
        duration: i32,
        mime_type: String,
        size: i32,
        dc_id: i32,
    },
}

#[derive(TLType)]
pub enum Document {
    #[tl_id(_36f8c871)] Empty {
        id: i64,
    },
    #[tl_id(_f9a39f4f)] Document {
        id: i64,
        access_hash: i64,
        date: i32,
        mime_type: String,
        size: i32,
        thumb: PhotoSize,
        dc_id: i32,
        attributes: Vector<DocumentAttribute>,
    },
}

#[derive(TLType)]
pub enum NotifyPeer {
    #[tl_id(_9fd40bd8)] Peer {
        peer: Peer,
    },
    #[tl_id(_b4c83b4c)] Users,
    #[tl_id(_c007cec3)] Chats,
    #[tl_id(_74d07c60)] All,
}

#[derive(TLType)]
pub enum SendMessageAction {
    #[tl_id(_16bf744e)] Typing,
    #[tl_id(_fd5ec8f5)] Cancel,
    #[tl_id(_a187d66f)] RecordVideo,
    #[tl_id(_92042ff7)] UploadVideo,
    #[tl_id(_d52f73f7)] RecordAudio,
    #[tl_id(_e6ac8a6f)] UploadAudio,
    #[tl_id(_990a3c1a)] UploadPhoto,
    #[tl_id(_8faee98e)] UploadDocument,
    #[tl_id(_176f8ba1)] GeoLocation,
    #[tl_id(_628cbc6f)] ChooseContact,
}

#[derive(TLType)]
#[tl_id(_ea879f95)]
pub struct ContactFound {
    pub user_id: i32,
}

#[derive(TLType)]
pub enum InputPrivacyKey {
    #[tl_id(_4f96cb18)] StatusTimestamp,
}

#[derive(TLType)]
pub enum PrivacyKey {
    #[tl_id(_bc2eab30)] StatusTimestamp,
}

#[derive(TLType)]
pub enum InputPrivacyRule {
    #[tl_id(_d09e07b)] AllowContacts,
    #[tl_id(_184b35ce)] AllowAll,
    #[tl_id(_131cc67f)] AllowUsers {
        users: Vector<InputUser>,
    },
    #[tl_id(_ba52007)] DisallowContacts,
    #[tl_id(_d66b66c9)] DisallowAll,
    #[tl_id(_90110467)] DisallowUsers {
        users: Vector<InputUser>,
    },
}

#[derive(TLType)]
pub enum PrivacyRule {
    #[tl_id(_fffe1bac)] AllowContacts,
    #[tl_id(_65427b82)] AllowAll,
    #[tl_id(_4d5bbe0c)] AllowUsers {
        users: Vector<i32>,
    },
    #[tl_id(_f888fa1a)] DisallowContacts,
    #[tl_id(_8b73e763)] DisallowAll,
    #[tl_id(_c7f49b7)] DisallowUsers {
        users: Vector<i32>,
    },
}

#[derive(TLType)]
#[tl_id(_554abb6f)]
pub struct PrivacyRules {
    pub rules: Vector<PrivacyRule>,
    pub users: Vector<User>,
}

#[derive(TLType)]
#[tl_id(_b8d0afdf)]
pub struct AccountDaysTTL {
    pub days: i32,
}

#[derive(TLType)]
pub enum DocumentAttribute {
    #[tl_id(_6c37c15c)] ImageSize {
        w: i32,
        h: i32,
    },
    #[tl_id(_11b58939)] Animated,
    #[tl_id(_fb0a5727)] Sticker,
    #[tl_id(_5910cccb)] Video {
        duration: i32,
        w: i32,
        h: i32,
    },
    #[tl_id(_51448e5)] Audio {
        duration: i32,
    },
    #[tl_id(_15590068)] Filename {
        file_name: String,
    },
}

#[derive(Debug, TLType)]
#[tl_id(_ae636f24)]
pub struct DisabledFeature {
    pub feature: String,
    pub description: String,
}

// End-to-end encryption types

#[derive(TLType)]
pub enum DecryptedMessage {
    // ==Layer 8==
    #[tl_id(_1f814f1f)] Message_v8 {
        random_id: i64,
        random_bytes: Vec<u8>,
        message: String,
        media: DecryptedMessageMedia,
    },
    #[tl_id(_aa48327d)] Service_v8 {
        random_id: i64,
        random_bytes: Vec<u8>,
        action: DecryptedMessageAction,
    },

    // ==Layer 17==
    #[tl_id(_204d3878)] Message_v17 {
        random_id: i64,
        ttl: i32,
        message: String,
        media: DecryptedMessageMedia,
    },
    #[tl_id(_73164160)] Service_v17 {
        random_id: i64,
        action: DecryptedMessageAction,
    },
}

#[derive(TLType)]
pub enum DecryptedMessageMedia {
    // ==Layer 8==
    #[tl_id(_89f5c4a)] Empty_v8,
    #[tl_id(_32798a8c)] Photo_v8 {
        thumb: Vec<u8>,
        thumb_w: i32,
        thumb_h: i32,
        w: i32,
        h: i32,
        size: i32,
        key: Vec<u8>,
        iv: Vec<u8>,
    },
    #[tl_id(_4cee6ef3)] Video_v8 {
        thumb: Vec<u8>,
        thumb_w: i32,
        thumb_h: i32,
        duration: i32,
        w: i32,
        h: i32,
        size: i32,
        key: Vec<u8>,
        iv: Vec<u8>,
    },
    #[tl_id(_35480a59)] GeoPoint_v8 {
        lat: f64,
        long: f64,
    },
    #[tl_id(_588a0a97)] Contact_v8 {
        phone_number: String,
        first_name: String,
        last_name: String,
        user_id: i32,
    },
    #[tl_id(_b095434b)] Document_v8 {
        thumb: Vec<u8>,
        thumb_w: i32,
        thumb_h: i32,
        file_name: String,
        mime_type: String,
        size: i32,
        key: Vec<u8>,
        iv: Vec<u8>,
    },
    #[tl_id(_6080758f)] Audio_v8 {
        duration: i32,
        size: i32,
        key: Vec<u8>,
        iv: Vec<u8>,
    },

    // ==Layer 17==
    #[tl_id(_524a415d)] Video_v17 {
        thumb: Vec<u8>,
        thumb_w: i32,
        thumb_h: i32,
        duration: i32,
        mime_type: String,
        w: i32,
        h: i32,
        size: i32,
        key: Vec<u8>,
        iv: Vec<u8>,
    },
    #[tl_id(_57e0a9cb)] Audio_v17 {
        duration: i32,
        mime_type: String,
        w: i32,
        h: i32,
        size: i32,
        key: Vec<u8>,
        iv: Vec<u8>,
    },

    // ==Layer 23==
    #[tl_id(_fa95b0dd)] ExternalDocument_v23 {
        id: i64,
        access_hash: i64,
        date: i32,
        mime_type: String,
        size: i32,
        thumb: PhotoSize,
        dc_id: i32,
        attributes: Vector<DocumentAttribute>,
    },
}

#[derive(TLType)]
pub enum DecryptedMessageAction {
    // ==Layer 8==
    #[tl_id(_a1733aec)] SetMessageTTL_v8 {
        ttl_seconds: i32,
    },
    #[tl_id(_c4f40be)] ReadMessages_v8 {
        random_ids: Vector<i64>,
    },
    #[tl_id(_65614304)] DeleteMessages_v8 {
        random_ids: Vector<i64>,
    },
    #[tl_id(_8ac1f475)] ScreenshotMessages_v8 {
        random_ids: Vector<i64>,
    },
    #[tl_id(_6719e45c)] FlushHistory_v8,

    // ==Layer 17==
    #[tl_id(_511110b0)] Resend_v17 {
        start_seq_no: i32,
        end_seq_no: i32,
    },
    #[tl_id(_f3048883)] NotifyLayer_v17 {
        layer: i32,
    },
    #[tl_id(_ccb27641)] Typing_v17 {
        action: SendMessageAction,
    },

    // ==Layer 20==
    #[tl_id(_f3c9611b)] RequestKey_v20 {
        exchange_id: i64,
        g_a: Vec<u8>,
    },
    #[tl_id(_6fe1735b)] AcceptKey_v20 {
        exchange_id: i64,
        g_b: Vec<u8>,
        key_fingerprint: i64,
    },
    #[tl_id(_dd05ec6b)] AbortKey_v20 {
        exchange_id: i64,
    },
    #[tl_id(_ec2e0b9b)] CommitKey_v20 {
        exchange_id: i64,
        key_fingerprint: i64,
    },
    #[tl_id(_a82fdd63)] Noop_v20,
}

#[derive(TLType)]
pub enum DecryptedMessageLayer {
    // ==Layer 17==
    #[tl_id(_1be31789)] Layer_v17 {
        random_bytes: Vec<u8>,
        layer: i32,
        in_seq_no: i32,
        out_seq_no: i32,
        message: DecryptedMessage,
    }
}

// #[tl_id(_)]
