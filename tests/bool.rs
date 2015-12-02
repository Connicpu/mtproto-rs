extern crate mtproto;
extern crate byteorder;

use std::io::Cursor;
use mtproto::tl::Bool;
use mtproto::tl::parsing::{ReadContext, WriteContext};
use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};

struct BoolTest {
    true_buffer: [u8; 4],
    false_buffer: [u8; 4],
}

impl BoolTest {
    fn new() -> BoolTest {
        let mut test: BoolTest = unsafe { std::mem::uninitialized() };
        LittleEndian::write_u32(&mut test.true_buffer, 0x997275b5);
        LittleEndian::write_u32(&mut test.false_buffer, 0xbc799737);
        test
    }
}

#[test]
fn bool_serialization() {
    let mut buffer: [u8; 4] = [0; 4];
    let correct = BoolTest::new();
    
    WriteContext::new(Cursor::new(&mut buffer[..])).write_generic(&Bool(true)).unwrap();
    assert_eq!(buffer, correct.true_buffer);
    
    WriteContext::new(Cursor::new(&mut buffer[..])).write_generic(&Bool(false)).unwrap();
    assert_eq!(buffer, correct.false_buffer);
}

#[test]
fn bool_deserialization() {
    let data = BoolTest::new();
    
    let true_value: Bool = ReadContext::new(Cursor::new(&data.true_buffer[..])).read_generic().unwrap();
    assert_eq!(true_value, Bool(true));
    
    let false_value: Bool = ReadContext::new(Cursor::new(&data.false_buffer[..])).read_generic().unwrap();
    assert_eq!(false_value, Bool(false));
}

