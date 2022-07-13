//! Interface for buzzer use.

use crate::ErrorCode;

pub trait BuzzerClient {
    /// Called when the current sound played by the buzzer has finished
    /// or it was stopped.
    fn buzzer_done(&self, status: Result<(), ErrorCode>);
}

pub trait Buzzer<'a> {
    /// Play a sound at a chosen frequency and for a chosen duration.
    /// Once the buzzer finishes buzzing, the `buzzer_done()` callback
    /// is called.
    /// If called while the buzzer is playing, the driver checks if the same
    /// application called. If so, we override the current frequency and duration
    /// with the new ones. If it is a different application asking to use the buzzer,
    /// the driver returns the error code `RESERVED`.
    /// Return values:
    ///
    /// - `Ok(())`: The attempt at starting the buzzer was successful.
    /// - `FAIL`: Cannot start the buzzer.
    /// - `RESERVED`: The buzzer is currently in use by another app.
    fn buzz(&self, frequency_hz: usize, duration_ms: usize) -> Result<(), ErrorCode>;

    /// Stop the sound currenty playing.
    /// After the buzzer is successfully stopped, the `buzzer_done()`
    /// callback is called.
    ///
    /// Return values:
    ///
    /// - `Ok(())`: The attempt at stopping the buzzer was successful.
    /// - `FAIL`: Cannot stop the buzzer.
    fn stop(&self) -> Result<(), ErrorCode>;

    /// Set the client to be used for callbacks of the Buzzer
    /// implementation.
    fn set_client(&self, client: &'a dyn BuzzerClient);
}
