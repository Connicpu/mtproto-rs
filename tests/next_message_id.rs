/*extern crate mtproto;
use std::thread::sleep;
use std::time::Duration;

#[test]
fn test_next_message_id() {
    let mut session = mtproto::rpc::Session::new();
    
    let mut prev_id = 0;
    for _ in 0..10 {
        for _ in 0..32000 {
            let new_id = session.next_message_id();
            assert!(new_id > prev_id);
            prev_id = new_id;
        }
        
        sleep(Duration::new(0, 000_060_000));
    }
}
*/
