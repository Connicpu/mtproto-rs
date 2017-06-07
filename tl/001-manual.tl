---types---

rpc_result#f35c6d01 req_msg_id:long result:Object = manual.RpcResult;
message msg_id:long seqno:int body:LengthAndObject = manual.Message;
msg_container#73f1f8dc messages:vector<manual.Message> = manual.MessageContainer;
gzip_packed#3072cfa1 packed_data:bytes = manual.GzipPacked;

encrypted salt:long session_id:long message_id:long seq_no:int payload:LengthAndObject = manual.Encrypted;
plain auth_key_id:long message_id:long payload:LengthAndObject = manual.Plain;

bind_auth_key_inner#75a3f765 nonce:long temp_auth_key_id:long perm_auth_key_id:long temp_session_id:long expires_at:int = manual.BindAuthKeyInner;
p_q_inner_data_temp#3c6a84d4 pq:string p:string q:string nonce:int128 server_nonce:int128 new_nonce:int256 expires_in:int = P_Q_inner_data;
