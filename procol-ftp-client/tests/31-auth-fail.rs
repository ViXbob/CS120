extern crate protocol_ftp_client;

use protocol_ftp_client::*;
use std::str;

#[test]
fn session_sample() {
  let mut tx_buff:[u8; 1024] = [0; 1024];
  let mut tx_count = 0;

  let mut ftp_reciver = FtpReceiver::new();

  ftp_reciver = ftp_reciver
    .try_advance("220 Service ready for new user.\r\n".as_bytes()).ok().unwrap()
    .send_login(&mut tx_buff, &mut tx_count, "user");

  ftp_reciver = ftp_reciver
    .try_advance("331 User name okay, need password for user.\r\n".as_bytes()).ok().unwrap()
    .send_password(&mut tx_buff, &mut tx_count, "11");

  ftp_reciver = ftp_reciver.try_advance("530 Authentication failed.\r\n".as_bytes()).err().unwrap();
  assert_eq!(ftp_reciver.take_error().unwrap(), FtpError::AuthFailed);

  ftp_reciver.to_transmitter().send_login(&mut tx_buff, &mut tx_count, "anonymous");

}
