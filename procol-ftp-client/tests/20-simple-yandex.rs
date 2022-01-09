extern crate protocol_ftp_client;

use protocol_ftp_client::*;
use std::str;
use std::net::Ipv4Addr;

#[test]
fn session_sample() {
  let mut tx_buff:[u8; 1024] = [0; 1024];
  let mut tx_count = 0;

  let mut ftp_reciver = FtpReceiver::new();

  ftp_reciver = ftp_reciver
    .try_advance("220 This is ftp0.ydx.freebsd.org - hosted at Yandex.\r\n".as_bytes()).ok().unwrap()
    .send_login(&mut tx_buff, &mut tx_count, "anonymous");
  assert_eq!(str::from_utf8(&tx_buff[0 .. tx_count]).unwrap(), "USER anonymous\r\n");

  ftp_reciver = ftp_reciver
    .try_advance("331 Please specify the password.\r\n".as_bytes()).ok().unwrap()
    .send_password(&mut tx_buff, &mut tx_count, "anonymous@nowhere.com");
  assert_eq!(str::from_utf8(&tx_buff[0 .. tx_count]).unwrap(), "PASS anonymous@nowhere.com\r\n");

  let banner = "230-\r
230-This is ftp0.ydx.FreeBSD.org, graciously hosted by Yandex.\r
230-\r
230-FreeBSD files can be found in the /pub/FreeBSD directory.\r
230-\r
230 Login successful.\r
";
  let mut ftp_transmitter = ftp_reciver.try_advance(banner.as_bytes()).ok().unwrap();

  ftp_reciver = ftp_transmitter.send_type_req(&mut tx_buff, &mut tx_count, DataMode::Binary);
  assert_eq!(str::from_utf8(&tx_buff[0 .. tx_count]).unwrap(), "TYPE I\r\n");
  ftp_transmitter = ftp_reciver.try_advance("200 Switching to Binary mode.\r\n".as_bytes()).ok().unwrap();
  assert_eq!(ftp_transmitter.get_type(), &DataMode::Binary);

  ftp_reciver = ftp_transmitter.send_system_req(&mut tx_buff, &mut tx_count);
  assert_eq!(str::from_utf8(&tx_buff[0 .. tx_count]).unwrap(), "SYST\r\n");
  ftp_transmitter = ftp_reciver.try_advance("215 UNIX Type: L8\r\n".as_bytes()).ok().unwrap();
  assert_eq!(ftp_transmitter.get_system(), (&"UNIX".to_string(), &"L8".to_string()));

  ftp_reciver = ftp_transmitter.send_pwd_req(&mut tx_buff, &mut tx_count);
  assert_eq!(str::from_utf8(&tx_buff[0 .. tx_count]).unwrap(), "PWD\r\n");
  ftp_transmitter = ftp_reciver.try_advance("257 \"/\" is the current directory\r\n".as_bytes()).ok().unwrap();
  assert_eq!(ftp_transmitter.get_wd(), "/");

  ftp_reciver = ftp_transmitter.send_cwd_req(&mut tx_buff, &mut tx_count, "/pub/FreeBSD/releases/ISO-IMAGES/10.3");
  assert_eq!(str::from_utf8(&tx_buff[0 .. tx_count]).unwrap(), "CWD /pub/FreeBSD/releases/ISO-IMAGES/10.3\r\n");
  ftp_transmitter = ftp_reciver.try_advance("250 Directory successfully changed.\r\n".as_bytes()).ok().unwrap();
  assert_eq!(ftp_transmitter.get_wd(), "/pub/FreeBSD/releases/ISO-IMAGES/10.3");

  ftp_reciver = ftp_transmitter.send_pasv_req(&mut tx_buff, &mut tx_count);
  assert_eq!(str::from_utf8(&tx_buff[0 .. tx_count]).unwrap(), "PASV\r\n");
  ftp_transmitter = ftp_reciver.try_advance("227 Entering Passive Mode (77,88,40,106,195,70).\r\n".as_bytes()).ok().unwrap();

  assert_eq!(ftp_transmitter.take_endpoint(), (Ipv4Addr::new(77, 88, 40, 106), 49990));

  ftp_reciver = ftp_transmitter.send_list_req(&mut tx_buff, &mut tx_count);
  assert_eq!(str::from_utf8(&tx_buff[0 .. tx_count]).unwrap(), "LIST -l\r\n");

  let listing = "-rw-r--r--    1 ftp      ftp          5430 Jul 19  2014 favicon.ico\r
-rw-r--r--    1 ftp      ftp           660 Nov 02  2015 index.html\r
drwxr-xr-x    3 ftp      ftp             3 Jul 19  2014 pub\r\n";


  let listing_tx = "150 Here comes the directory listing.\r\n";

  ftp_transmitter = ftp_reciver.try_advance(listing_tx.as_bytes()).ok().unwrap();
  let list = ftp_transmitter.parse_list(listing.as_bytes()).unwrap();
  assert_eq!(list.len(), 3);
  assert_eq!(list[0], RemoteFile { kind: RemoteFileKind::File, size: 5430,  name: "favicon.ico".to_string() } );
  assert_eq!(list[1], RemoteFile { kind: RemoteFileKind::File, size: 660,  name: "index.html".to_string() } );
  assert_eq!(list[2], RemoteFile { kind: RemoteFileKind::Directory, size: 3,  name: "pub".to_string() } );

  ftp_transmitter = ftp_transmitter.to_receiver().try_advance("226 Directory send OK.\r\n".as_bytes())
    .ok().unwrap();

  ftp_reciver = ftp_transmitter.send_get_req(&mut tx_buff, &mut tx_count, "/a/b/favicon.ico");
  assert_eq!(str::from_utf8(&tx_buff[0 .. tx_count]).unwrap(), "RETR /a/b/favicon.ico\r\n");

  let _ = ftp_reciver.try_advance("150 Opening BINARY mode data connection for /a/b/favicon.ico (4259 bytes).\r\n".as_bytes())
    .ok().unwrap()
    .to_receiver().try_advance("226 Transfer complete\r\n".as_bytes()).ok().unwrap();
  ;

}
