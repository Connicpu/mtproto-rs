extern crate mtproto;
extern crate byteorder;

use std::io::Cursor;
use mtproto::schema::Bool;
use mtproto::tl::parsing::{Reader, ReadContext, Writer, WriteContext};
use byteorder::{ByteOrder, LittleEndian};

#[derive(Default)]
struct BoolTest {
    true_buffer: [u8; 4],
    false_buffer: [u8; 4],
}

impl BoolTest {
    fn new() -> BoolTest {
        let mut test: BoolTest = Default::default();
        LittleEndian::write_u32(&mut test.true_buffer, 0x997275b5);
        LittleEndian::write_u32(&mut test.false_buffer, 0xbc799737);
        test
    }
}

#[test]
fn bool_serialization() {
    let mut buffer: [u8; 4] = [0; 4];
    let correct = BoolTest::new();

    WriteContext::new(Cursor::new(&mut buffer[..])).write_tl(&Bool::boolTrue).unwrap();
    assert_eq!(buffer, correct.true_buffer);

    WriteContext::new(Cursor::new(&mut buffer[..])).write_tl(&Bool::boolFalse).unwrap();
    assert_eq!(buffer, correct.false_buffer);
}

#[test]
fn bool_deserialization() {
    let data = BoolTest::new();

    let true_value: Bool = ReadContext::new(Cursor::new(&data.true_buffer[..])).read_tl().unwrap();
    match true_value {
        Bool::boolTrue => {},
        other => panic!("Bool::boolTrue deserialization failed: actual value {:?}", other),
    }

    let false_value: Bool = ReadContext::new(Cursor::new(&data.false_buffer[..])).read_tl().unwrap();
    match false_value {
        Bool::boolFalse => {},
        other => panic!("Bool::boolFalse deserialization failed: actual value {:?}", other),
    }
}
