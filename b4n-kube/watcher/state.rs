/// Background observer connection state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BgObserverState {
    Idle,
    Connecting,
    Reconnecting,
    Connected,
    Ready,
}

impl BgObserverState {
    /// Returns `true` if state indicates that observer is or was connected.
    pub fn is_connected(&self) -> bool {
        matches!(self, BgObserverState::Connected | BgObserverState::Ready)
    }
}

impl From<u8> for BgObserverState {
    fn from(value: u8) -> Self {
        match value {
            1 => BgObserverState::Connecting,
            2 => BgObserverState::Reconnecting,
            3 => BgObserverState::Connected,
            4 => BgObserverState::Ready,
            _ => BgObserverState::Idle,
        }
    }
}

impl From<BgObserverState> for u8 {
    fn from(value: BgObserverState) -> Self {
        match value {
            BgObserverState::Idle => 0,
            BgObserverState::Connecting => 1,
            BgObserverState::Reconnecting => 2,
            BgObserverState::Connected => 3,
            BgObserverState::Ready => 4,
        }
    }
}

/// Background observer connection health.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BgObserverHealth {
    Good,
    ConnectionError,
    ApiError,
}

impl BgObserverHealth {
    /// Returns error health state.
    pub fn error(is_api_error: bool) -> Self {
        if is_api_error {
            BgObserverHealth::ApiError
        } else {
            BgObserverHealth::ConnectionError
        }
    }
}

impl From<u8> for BgObserverHealth {
    fn from(value: u8) -> Self {
        match value {
            1 => BgObserverHealth::ConnectionError,
            2 => BgObserverHealth::ApiError,
            _ => BgObserverHealth::Good,
        }
    }
}

impl From<BgObserverHealth> for u8 {
    fn from(value: BgObserverHealth) -> Self {
        match value {
            BgObserverHealth::Good => 0,
            BgObserverHealth::ConnectionError => 1,
            BgObserverHealth::ApiError => 2,
        }
    }
}
