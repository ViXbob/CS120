#[macro_use] extern crate lazy_static;
extern crate regex;

use regex::Regex;
use std::str;
use std::fmt;
use std::rc::Rc;
use std::net::Ipv4Addr;
use std::ptr;

const OPENNING_DATA_CONNECTION:u32 = 150;
const OPERATION_SUCCESS:u32        = 200;
const SYSTEM_RECEIVED:u32          = 215;
const LOGGED_EXPECTED:u32          = 220;
const CLOSING_DATA_CONNECTION:u32  = 226;
const PASSIVE_MODE:u32             = 227;
const LOGGED_IN:u32                = 230;
const CWD_CONFIRMED:u32            = 250;
const PATHNAME_AVAILABLE:u32       = 257;
const PASSWORD_EXPECTED:u32        = 331;
const AUTHENTICATION_FAILED:u32    = 530;


/// Defines data transfer mode: binary (aka image) or text.
#[derive(Clone)]
#[derive(PartialEq)]
#[derive(Debug)]
pub enum DataMode {
  Binary,
  Text
}

impl fmt::Display for DataMode {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      &DataMode::Binary => write!(f, "data-mode:binary"),
      &DataMode::Text   => write!(f, "data-mode:text"),
    }
  }
}

enum State {
  NonAuthorized,
  Authorized,
  LoginReady,
  LoginReqSent,

  PasswordExpected,
  PasswordReqSent,

  PwdReqSent,
  PathReceived(String),

  CwdReqSent(String),
  CwdConfirmed,

  DataTypeReqSent(DataMode),
  DataTypeConfirmed(DataMode),

  SystemReqSent,
  SystemRecived(String, String),

  PassiveReqSent,
  PassiveConfirmed(Ipv4Addr, u16),

  ListReqSent,
  FileReqSent,

  DataTransferStarted,
  DataTransferCompleted,
}

impl fmt::Display for State {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      &State::PathReceived(ref value)              => write!(f, "[state: path-received({})]", value),
      &State::CwdReqSent(ref value)                => write!(f, "[state: cwd-req-sent({})]", value),
      &State::DataTypeReqSent(ref value)           => write!(f, "[state: data-type-req-sent({})]", value),
      &State::DataTypeConfirmed(ref value)         => write!(f, "[state: data-type-confirmed({})]", value),
      &State::SystemRecived(ref name, ref subtype) => write!(f, "[state: system-recieved({}/{})]", name, subtype),
      &State::PassiveConfirmed(ref addr, ref port) => write!(f, "[state: passive-mode ({}:{})]", addr, port),
      _ => {
        let state = match self {
          &State::NonAuthorized         => "non-authorized",
          &State::Authorized            => "authorized",
          &State::LoginReady            => "login-ready",
          &State::LoginReqSent          => "login-req-sent",
          &State::PasswordExpected      => "password-expected",
          &State::PasswordReqSent       => "password-req-sent",
          &State::PwdReqSent            => "pwd-req-sent",
          &State::SystemReqSent         => "system-req-sent",
          &State::PassiveReqSent        => "passive-req-sent",
          &State::ListReqSent           => "list-req-sent",
          &State::FileReqSent           => "file-req-sent",
          &State::DataTransferStarted   => "data-transfer-started",
          &State::DataTransferCompleted => "data-transfer-completed",
          &State::CwdConfirmed          => "cwd-confirmed",
          _ => unreachable!(),
        };
        write!(f, "[state: {}]", state)
      }
    }
  }
}

/// Defines files type for parsed `LIST` command.
#[derive(PartialEq)]
#[derive(Debug)]
pub enum RemoteFileKind {
  File,
  Directory,
}

/// Represents single item parsed `LIST` command.
#[derive(PartialEq)]
#[derive(Debug)]
pub struct RemoteFile {
  pub kind: RemoteFileKind,
  pub size: usize,
  pub name: String,
}

#[derive(PartialEq)]
/// Error occured in parsing FTP data.
pub enum FtpError {
  /// No enough data has been provided.
  NotEnoughData,
  /// Protocol error occur, i.e. got `A` while expected `B`.
  ProtocolError(String),
  /// Some meaningless data.
  GarbageData,
  /// Failed to authenticate.
  AuthFailed,
}

impl fmt::Display for FtpError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      &FtpError::NotEnoughData        => { write!(f, "[no enough data]") }
      &FtpError::AuthFailed           => { write!(f, "[authorization failed]") }
      &FtpError::GarbageData          => { write!(f, "[garbage]") }
      &FtpError::ProtocolError(ref s) => { write!(f, "[protocol error: {}]", s) }
    }
  }
}

struct FtpInternals {
  error: Option<FtpError>,
  data_mode: Option<DataMode>,
  working_dir: Option<String>,
  sent_request: Option<Rc<State>>,
  system: Option<(String, String)>,
  endpoint: Option<(Ipv4Addr, u16)>,
  state: Rc<State>,
}

/// "Passive" side of FTP protocol, which mean that receiver expects
/// some data from remote server. As soon as it receives enough data
/// it can "advance" to transmitter state, i.e. fill buffer with
/// commands to be further sent to the remote server.
pub struct FtpReceiver {
  internals: Rc<FtpInternals>
}

/// "Active" side of FTP protocol, i.e. fill buffer with desired
/// FTP commands for further delivery to remote server.
pub struct FtpTransmitter {
  internals: Rc<FtpInternals>
}


impl fmt::Debug for FtpError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &FtpError::GarbageData            => write!(f, "garbage data"),
            &FtpError::NotEnoughData          => write!(f, "no enough data"),
            &FtpError::ProtocolError(ref err) => write!(f, "protocol error: {}", err),
            &FtpError::AuthFailed             => write!(f, "authentication failed"),
        }
    }
}

impl FtpReceiver {
  pub fn new() -> Self {
    FtpReceiver {
      internals: Rc::new(FtpInternals {
        error: None,
        data_mode: None,
        working_dir: None,
        sent_request: None,
        system: None,
        endpoint: None,
        state: Rc::new(State::NonAuthorized),
      })
    }
  }


  fn advance_state(prev_state: &State, prev_req: &Option<Rc<State>>, bytes: &[u8]) -> Result<State, FtpError> {

    lazy_static! {
      static ref RE_RESPONCE_CODE: Regex = Regex::new("(?m:^(\\d{3}) (.+)\r$)").unwrap();
      static ref RE_PATHNAME: Regex = Regex::new("\"(.+)\"").unwrap();
      static ref RE_SYSTEM: Regex = Regex::new("(\\w+) [Tt]ype: (\\w+)").unwrap();
      static ref RE_PARTRIAL_RESPONCE_CODE: Regex = Regex::new("(?m:^(\\d{3})-.+\r$)").unwrap();
      static ref RE_PASSIVE_MODE: Regex = Regex::new("Entering Passive Mode \\((\\d+),(\\d+),(\\d+),(\\d+),(\\d+),(\\d+)\\)").unwrap();
    }

    str::from_utf8(bytes)
      .map_err(|_| FtpError::GarbageData)
      .and_then(|response|
        RE_RESPONCE_CODE.captures(&response)
          .ok_or_else(||{
            if RE_PARTRIAL_RESPONCE_CODE.is_match(response) {
              FtpError::NotEnoughData
            } else {
              FtpError::GarbageData
            }
          })
          .and_then(|captures| {
            let code_str = captures.at(1).unwrap();
            let code:u32 = code_str.parse().unwrap();
            match code {
              LOGGED_EXPECTED          => Ok(State::LoginReady),
              PASSWORD_EXPECTED        => Ok(State::PasswordExpected),
              LOGGED_IN                => Ok(State::Authorized),
              AUTHENTICATION_FAILED    => Err(FtpError::AuthFailed),
              OPENNING_DATA_CONNECTION => Ok(State::DataTransferStarted),
              CLOSING_DATA_CONNECTION  => Ok(State::DataTransferCompleted),
              CWD_CONFIRMED            => Ok(State::CwdConfirmed),
              OPERATION_SUCCESS  => {
                match &*prev_req {
                  &Some(ref prev_sent_req) => {
                    match &**prev_sent_req {
                      &State::DataTypeReqSent(ref value) => Ok(State::DataTypeConfirmed(value.clone())),
                      _ => Err(FtpError::GarbageData),
                    }
                  },
                  _ => Err(FtpError::GarbageData),
                }
              },
              PATHNAME_AVAILABLE => {
                let pathname_str = captures.at(2).unwrap();
                RE_PATHNAME.captures(pathname_str)
                  .ok_or(FtpError::GarbageData)
                  .and_then(|path_capture|{
                    let path = path_capture.at(1).unwrap();
                    Ok(State::PathReceived(path.to_string()))
                  })
              },
              SYSTEM_RECEIVED => {
                let system_str = captures.at(2).unwrap();
                RE_SYSTEM.captures(system_str)
                  .ok_or(FtpError::GarbageData)
                  .and_then(|path_capture|{
                    let name = path_capture.at(1).unwrap();
                    let subtype = path_capture.at(2).unwrap();
                    Ok(State::SystemRecived(name.to_string(), subtype.to_string()))
                  })
              },
              PASSIVE_MODE => {
                let addr_str = captures.at(2).unwrap();
                RE_PASSIVE_MODE.captures(addr_str)
                  .ok_or(FtpError::GarbageData)
                  .and_then(|path_capture|{
                    let mut numbers = path_capture.iter().skip(1).map(|opt_value| {
                      let value = opt_value.unwrap();
                      let number:u8 = value.parse().unwrap();
                      number
                    });
                    let a = numbers.next().unwrap();
                    let b = numbers.next().unwrap();
                    let c = numbers.next().unwrap();
                    let d = numbers.next().unwrap();
                    let p1 = numbers.next().unwrap();
                    let p2 = numbers.next().unwrap();

                    let p1_16 = p1 as u16;
                    let p2_16 = p2 as u16;

                    let addr = Ipv4Addr::new(a, b, c, d);
                    let port = 256 * p1_16 + p2_16;
                    Ok(State::PassiveConfirmed(addr, port))
                  })
              }
              _ => panic!(format!("unknown responce: {}", response))
            }
          })
      )
      .and_then(|new_state|{
        let allowed:bool = match (prev_state, &new_state) {
          (&State::NonAuthorized, &State::LoginReady)                  => true,
          (&State::LoginReqSent, &State::PasswordExpected)             => true,
          (&State::PasswordExpected, &State::PasswordReqSent)          => true,
          (&State::PasswordReqSent, &State::Authorized)                => true,
          (&State::PwdReqSent, &State::PathReceived(_))                => true,
          (&State::DataTypeReqSent(_), &State::DataTypeConfirmed(_))   => true,
          (&State::SystemReqSent, &State::SystemRecived(_, _))         => true,
          (&State::PassiveReqSent, &State::PassiveConfirmed(_, _))     => true,
          (&State::ListReqSent, &State::DataTransferStarted)           => true,
          (&State::FileReqSent, &State::DataTransferStarted)           => true,
          (&State::DataTransferStarted, &State::DataTransferCompleted) => true,
          (&State::CwdReqSent(_), &State::CwdConfirmed)                => true,
          _ => false,
        };
        if allowed {
          Ok(new_state)
        } else {
          println!("transition {} => {} is not allowed", prev_state, new_state);
          Err(FtpError::ProtocolError(format!("{} => {} is not allowed", prev_state, new_state)))
        }
      })
  }


  /// Try to consume `Receiver` by parsing buffer and advance into `Transmitter`.
  /// In the case of an error, it returns unmodified `Receiver` as the error. The
  /// actually happened error can be obtained via `take_error`.
  ///
  /// In case of success it remembers the last successful state, probably switches
  /// it and returns `Transmitter` object.
  pub fn try_advance(self, buffer: &[u8]) -> Result<FtpTransmitter, Self> {
    let mut internals = self.internals;

    let transition_result = FtpReceiver::advance_state(&internals.state, &internals.sent_request, buffer);

    match transition_result {
      Err(e) => {
        println!("error on state: {}", internals.state);
        if &e == &FtpError::AuthFailed {
          Rc::get_mut(&mut internals).unwrap().state = Rc::new(State::LoginReady);
        }
        Rc::get_mut(&mut internals).unwrap().error = Some(e);
        Err(FtpReceiver { internals: internals.clone() })
      }
      ,
      Ok(new_state) => {
        {
          let mut int_ref = Rc::get_mut(&mut internals).unwrap();
          let sent_request = int_ref.sent_request.clone();

          let final_state = match new_state {
            State::PathReceived(path) => {
              int_ref.working_dir = Some(path);
              State::Authorized
            },
            State::DataTypeConfirmed(data_type) => {
              int_ref.data_mode = Some(data_type);
              State::Authorized
            },
            State::SystemRecived(name, subtype) => {
              int_ref.system = Some((name, subtype));
              State::Authorized
            }
            State::PassiveConfirmed(addr, port) => {
              int_ref.endpoint = Some((addr, port));
              State::Authorized
            }
            State::CwdConfirmed => {
              int_ref.working_dir = match &sent_request.unwrap().as_ref() {
                &&State::CwdReqSent(ref path) => Some(path.clone()),
                _ => { unreachable!() },
              };
              State::Authorized
            }
            State::DataTransferCompleted => {
              State::Authorized
            }
            _ => new_state,
          };

          int_ref.state = Rc::new(final_state);
          int_ref.sent_request = None;
        }
        Ok(FtpTransmitter { internals: internals })
      }
    }
  }

  /// Returns tha last occurred error, and internally
  /// sets up `None`.
  pub fn take_error(&mut self) -> Option<FtpError> {
    Rc::get_mut(&mut self.internals).unwrap().error.take()
  }

  /// Sometimes you need to manually advance to `Transmitter`
  /// e.g. in case of Authorization Error, you can re-send
  /// other credentials.
  pub fn to_transmitter(self) -> FtpTransmitter {
    FtpTransmitter { internals: self.internals }
  }

}


lazy_static! {
  static ref DATA_USER: &'static [u8]        = "USER ".as_bytes();
  static ref DATA_PASS: &'static [u8]        = "PASS ".as_bytes();
  static ref DATA_PWD: &'static [u8]         = "PWD\r\n".as_bytes();
  static ref DATA_ENDING: &'static [u8]      = "\r\n".as_bytes();
  static ref DATA_DATA_BINARY: &'static [u8] = "TYPE I\r\n".as_bytes();
  static ref DATA_DATA_TEXT: &'static [u8]   = "TYPE A\r\n".as_bytes();
  static ref DATA_SYST: &'static [u8]        = "SYST\r\n".as_bytes();
  static ref DATA_PASV: &'static [u8]        = "PASV\r\n".as_bytes();
  static ref DATA_LIST: &'static [u8]        = "LIST -l\r\n".as_bytes();
  static ref DATA_CWD:  &'static [u8]        = "CWD ".as_bytes();
  static ref DATA_RETR: &'static [u8]        = "RETR ".as_bytes();
}


impl FtpTransmitter {

  /// Sometimes you need to manually advance to `Receiver`
  /// e.g. in case of `LIST` or file get commands, servers sends
  /// start data transfer and end data transfer responses.
  pub fn to_receiver(self) -> FtpReceiver {
    FtpReceiver { internals: self.internals }
  }

  /// Fills the output buffer with the login command (takes `login` string argument ),
  /// modifies `count` variable with the count of written bytes and returns `FtpReceiver`.
  pub fn send_login(self, buffer: &mut [u8], count: &mut usize, login: &str) -> FtpReceiver {
    let mut internals = self.internals;
    let current_state = internals.state.clone();

    match &*internals.state {

      &State::LoginReady => {
        let data_login = login.as_bytes();
        let mut my_count = 0;
        unsafe {
          ptr::copy_nonoverlapping(&DATA_USER[0], &mut buffer[my_count], DATA_USER.len());
          my_count += DATA_USER.len();
          ptr::copy_nonoverlapping(&data_login[0], &mut buffer[my_count], data_login.len());
          my_count += data_login.len();
          ptr::copy_nonoverlapping(&DATA_ENDING[0], &mut buffer[my_count], DATA_ENDING.len());
        };
        my_count += DATA_ENDING.len();
        *count = my_count;
        {
          let mut int_ref = Rc::get_mut(&mut internals).unwrap();
          int_ref.state = Rc::new(State::LoginReqSent);
          int_ref.sent_request = Some(int_ref.state.clone());
        }

        FtpReceiver { internals: internals }
      },
      _ => panic!(format!("send_login is not allowed from the {}" , current_state)),
    }
  }

  /// Fills the output buffer with the password command (takes `password` string argument ),
  /// modifies `count` variable with the count of written bytes and returns `FtpReceiver`.
  pub fn send_password(self, buffer: &mut [u8], count: &mut usize, pass: &str) -> FtpReceiver {
    let mut internals = self.internals;

    match &*internals.state {
      &State::PasswordExpected => {
        let data_password = pass.as_bytes();
        let mut my_count = 0;
        unsafe {
          ptr::copy_nonoverlapping(&DATA_PASS[0], &mut buffer[my_count], DATA_PASS.len());
          my_count += DATA_PASS.len();
          ptr::copy_nonoverlapping(&data_password[0], &mut buffer[my_count], data_password.len());
          my_count += data_password.len();
          ptr::copy_nonoverlapping(&DATA_ENDING[0], &mut buffer[my_count], DATA_ENDING.len());
        };
        my_count += DATA_ENDING.len();
        *count = my_count;
        {
          let mut int_ref = Rc::get_mut(&mut internals).unwrap();
          int_ref.state = Rc::new(State::PasswordReqSent);
          int_ref.sent_request = Some(int_ref.state.clone());
        }

        FtpReceiver { internals: internals }
      },
      _ => panic!("send_password is not allowed from the current state"),
    }
  }

  /// Fills the output buffer with the PWD command (take current working directory on remote server),
  /// modifies `count` variable with the count of written bytes and returns `FtpReceiver`.
  pub fn send_pwd_req(self, buffer: &mut [u8], count: &mut usize) -> FtpReceiver {
    let mut internals = self.internals;

    match &*internals.state {
      &State::Authorized => {
        unsafe { ptr::copy_nonoverlapping(&DATA_PWD[0], &mut buffer[0], DATA_PWD.len()); }
        *count = DATA_PWD.len();
        {
          let mut int_ref = Rc::get_mut(&mut internals).unwrap();
          int_ref.state = Rc::new(State::PwdReqSent);
          int_ref.sent_request = Some(int_ref.state.clone());
        }

        FtpReceiver { internals: internals }
      },
      _ => panic!("send_pwd_req is not allowed from the current state"),
    }
  }

  /// Returns current working directory. Assumes either that  `send_pwd_req` or `send_cwd_req`
  /// has been sent and succeeded.
  pub fn get_wd(&self) -> &str {
    match &self.internals.working_dir {
      &Some(ref path) => &path,
      &None           => panic!("get_wd is not available (did you called send_pwd_req?)"),
    }
  }

  /// Fills the output buffer with the data transfer mode request (binary or text),
  /// modifies `count` variable with the count of written bytes and returns `FtpReceiver`.
  pub fn send_type_req(self, buffer: &mut [u8], count: &mut usize, data_type: DataMode) -> FtpReceiver {
    let mut internals = self.internals;

    match &*internals.state {
      &State::Authorized => {
        match &data_type {
          &DataMode::Binary => {
            unsafe { ptr::copy_nonoverlapping(&DATA_DATA_BINARY[0], &mut buffer[0], DATA_DATA_BINARY.len()); }
            *count = DATA_DATA_BINARY.len();
          },
          &DataMode::Text => {
            unsafe { ptr::copy_nonoverlapping(&DATA_DATA_TEXT[0], &mut buffer[0], DATA_DATA_TEXT.len()); }
            *count = DATA_DATA_TEXT.len();
          }
        };
        {
          let mut int_ref = Rc::get_mut(&mut internals).unwrap();
          int_ref.state = Rc::new(State::DataTypeReqSent(data_type));
          int_ref.sent_request = Some(int_ref.state.clone());
        }

        FtpReceiver { internals: internals }
      },
      _ => panic!("send_type_req is not allowed from the {}", internals.state),
    }
  }

  /// Returns current data mode. Assumes either that  `send_type_req`
  /// has been sent and succeeded.
  pub fn get_type(&self) -> &DataMode {
    match &self.internals.data_mode {
      &Some(ref mode) => &mode,
      &None           => panic!("get_type is not available (did you called send_type_req?)"),
    }
  }

  /// Fills the output buffer with the remote system request;
  /// modifies `count` variable with the count of written bytes and returns `FtpReceiver`.
  pub fn send_system_req(self, buffer: &mut [u8], count: &mut usize) -> FtpReceiver {
    let mut internals = self.internals;

    match &*internals.state {
      &State::Authorized => {
        unsafe { ptr::copy_nonoverlapping(&DATA_SYST[0], &mut buffer[0], DATA_SYST.len()); }
        *count = DATA_SYST.len();
        {
          let mut int_ref = Rc::get_mut(&mut internals).unwrap();
          int_ref.state = Rc::new(State::SystemReqSent);
          int_ref.sent_request = Some(int_ref.state.clone());
        }

        FtpReceiver { internals: internals }
      },
      _ => panic!("send_type_req is not allowed from the {}", internals.state),
    }
  }

  /// Returns remote system with subtype. Assumes either that `send_system_req`
  /// has been sent and succeeded.
  pub fn get_system(&self) -> (&String, &String) {
    match &self.internals.system {
      &Some((ref name, ref subtype)) => (&name, &subtype),
      &None                          => panic!("get_system is not available (did you called send_system_req?)"),
    }
  }

  /// Fills the output buffer with the PASS requests to allow further data transfer (`LIST` or get file)
  /// modifies `count` variable with the count of written bytes and returns `FtpReceiver`.
  pub fn send_pasv_req(self, buffer: &mut [u8], count: &mut usize) -> FtpReceiver {
    let mut internals = self.internals;

    match &*internals.state {
      &State::Authorized => {
        unsafe { ptr::copy_nonoverlapping(&DATA_PASV[0], &mut buffer[0], DATA_PASV.len()); }
        *count = DATA_PASV.len();
        {
          let mut int_ref = Rc::get_mut(&mut internals).unwrap();
          int_ref.state = Rc::new(State::PassiveReqSent);
          int_ref.sent_request = Some(int_ref.state.clone());
        }

        FtpReceiver { internals: internals }
      },
      _ => panic!("send_pasv_req is not allowed from the {}", internals.state),
    }
  }

  /// Fills the output buffer with get remove file command (takes `path` string argument ),
  /// modifies `count` variable with the count of written bytes and returns `FtpReceiver`.
  pub fn send_get_req(self, buffer: &mut [u8], count: &mut usize, file_path: &str) -> FtpReceiver {
    let mut internals = self.internals;

    match &*internals.state {
      &State::Authorized => {
        let data_path = file_path.as_bytes();
        let mut my_count = 0;
        unsafe {
          ptr::copy_nonoverlapping(&DATA_RETR[0], &mut buffer[my_count], DATA_RETR.len());
          my_count += DATA_RETR.len();
          ptr::copy_nonoverlapping(&data_path[0], &mut buffer[my_count], data_path.len());
          my_count += data_path.len();
          ptr::copy_nonoverlapping(&DATA_ENDING[0], &mut buffer[my_count], DATA_ENDING.len());
        };
        my_count += DATA_ENDING.len();
        {
          let mut int_ref = Rc::get_mut(&mut internals).unwrap();
          int_ref.state = Rc::new(State::FileReqSent);
          int_ref.sent_request = Some(int_ref.state.clone());
        }
        *count = my_count;

        FtpReceiver { internals: internals }
      },
      _ => panic!("send_get_req is not allowed from the {}", internals.state),
    }
  }


  /// Fills the output buffer with change remote working directory command (takes `path` string argument ),
  /// modifies `count` variable with the count of written bytes and returns `FtpReceiver`.
  pub fn send_cwd_req(self, buffer: &mut [u8], count: &mut usize, path: &str) -> FtpReceiver {
    let mut internals = self.internals;

    match &*internals.state {
      &State::Authorized => {
        let data_path = path.as_bytes();
        let mut my_count = 0;
        unsafe {
          ptr::copy_nonoverlapping(&DATA_CWD[0], &mut buffer[my_count], DATA_CWD.len());
          my_count += DATA_CWD.len();
          ptr::copy_nonoverlapping(&data_path[0], &mut buffer[my_count], path.len());
          my_count += path.len();
          ptr::copy_nonoverlapping(&DATA_ENDING[0], &mut buffer[my_count], DATA_ENDING.len());
        };
        my_count += DATA_ENDING.len();
        {
          let mut int_ref = Rc::get_mut(&mut internals).unwrap();
          int_ref.state = Rc::new(State::CwdReqSent(path.to_string()));
          int_ref.sent_request = Some(int_ref.state.clone());
        }
        *count = my_count;

        FtpReceiver { internals: internals }
      },
      _ => panic!("send_cwd_req is not allowed from the {}", internals.state),
    }
  }


  /// Takes pair of IP-address and port, where TCP-connection can be opened to.
  /// Assumes `send_pasv_req` has been invoked before.
  pub fn take_endpoint(&mut self) -> (Ipv4Addr, u16) {
    match Rc::get_mut(&mut self.internals).unwrap().endpoint.take() {
      Some((addr, port)) => (addr, port),
      None              => panic!("take_endpoint is not available (did you called send_pass_req?)"),
    }
  }

  /// Fills the output buffer with `LIST` command to get directory listing of current remote working directory;
  /// modifies `count` variable with the count of written bytes and returns `FtpReceiver`.
  pub fn send_list_req(self, buffer: &mut [u8], count: &mut usize) -> FtpReceiver {
    let mut internals = self.internals;

    match &*internals.state {
      &State::Authorized => {
          unsafe { ptr::copy_nonoverlapping(&DATA_LIST[0], &mut buffer[0], DATA_LIST.len()); }
          *count = DATA_LIST.len();
          {
            let mut int_ref = Rc::get_mut(&mut internals).unwrap();
            int_ref.state = Rc::new(State::ListReqSent);
            int_ref.sent_request = Some(int_ref.state.clone());
          }
          FtpReceiver { internals: internals }
        },
      _ => panic!("send_pass_req is not allowed from the {}", internals.state),
    }
  }

  /// Parses remote directory listing, requested by `send_list_req` command.
  pub fn parse_list(&self, data: &[u8]) -> Result<Vec<RemoteFile>, FtpError> {

    lazy_static! {
      static ref RE_LINE: Regex = Regex::new("(?m:^(.+)\r$)").unwrap();
      static ref RE_FILE: Regex = Regex::new("^([d-])(?:[rwx-]{3}){3} +\\d+ +\\w+ +\\w+ +(\\d+) +(.+) +(.+)$").unwrap();
    }
    str::from_utf8(data)
      .map_err(|_| FtpError::GarbageData)
      .and_then(|list|{
        let line_captures = RE_LINE.captures_iter(list);
        let files = line_captures
          .filter_map(|line_cap| {
            let line = line_cap.at(1).unwrap();
            // println!("line = {}", line);
            match RE_FILE.captures(line) {
              None => None,
              Some(captures) => {
                let kind_str = captures.at(1).unwrap();
                let size_str = captures.at(2).unwrap();
                let name = captures.at(4).unwrap();
                let kind = match kind_str {
                  "d" => RemoteFileKind::Directory,
                  "-" => RemoteFileKind::File,
                  _   => unreachable!(),
                };
                let size:usize = size_str.parse().unwrap();
                // println!("remote file: {} ({})", name, size);
                let remote_file = RemoteFile {
                  kind: kind,
                  size: size,
                  name: name.to_string(),
                };
                Some(remote_file)
              }
            }
          });
        let mut vec:Vec<RemoteFile> = Vec::new();
        for file in files {
          vec.push(file);
        }
        Ok(vec)
      })
  }

}
