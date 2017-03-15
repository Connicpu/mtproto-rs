---types---

rpc_result#f35c6d01 req_msg_id:long result:Object = manual.RpcResult;
message msg_id:long seqno:int body:LengthAndObject = manual.Message;
msg_container#73f1f8dc messages:vector<manual.Message> = manual.MessageContainer;
gzip_packed#3072cfa1 packed_data:bytes = manual.GzipPacked;

encrypted salt:long session_id:long message_id:long seq_no:int payload:LengthAndObject = manual.Encrypted;
plain auth_key_id:long message_id:long payload:LengthAndObject = manual.Plain;
