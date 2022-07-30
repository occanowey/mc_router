use std::{error::Error, fmt, io};

#[derive(Debug)]
pub enum ClientError {
    Proto(mcproto::error::Error),
    IO(io::Error),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ClientError - ")?;

        match self {
            ClientError::Proto(err) => write!(f, "Proto: {:?}", err),
            ClientError::IO(err) => write!(f, "IO: {:?}", err),
        }
    }
}

impl Error for ClientError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ClientError::Proto(err) => Some(err),
            ClientError::IO(err) => Some(err),
        }
    }
}

impl From<mcproto::error::Error> for ClientError {
    fn from(error: mcproto::error::Error) -> Self {
        ClientError::Proto(error)
    }
}

impl From<io::Error> for ClientError {
    fn from(error: io::Error) -> Self {
        ClientError::IO(error)
    }
}
