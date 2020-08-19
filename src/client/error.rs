use std::{error::Error, fmt, io};

#[derive(Debug)]
pub enum ClientError {
    InvalidHandshake(&'static str),
    IO(io::Error),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ClientError - ")?;

        match self {
            ClientError::InvalidHandshake(reason) => write!(f, "Invalid Handshake: {}", reason),
            ClientError::IO(io_err) => write!(f, "IO: {:?}", io_err),
        }
    }
}

impl Error for ClientError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ClientError::InvalidHandshake(_) => None,
            ClientError::IO(io_err) => Some(io_err),
        }
    }
}

impl From<io::Error> for ClientError {
    fn from(error: io::Error) -> Self {
        ClientError::IO(error)
    }
}
