#![deny(unexpected_cfgs)]

//! After `start_low_power_periodic_measurement()` the driver must reject
//! the commands that the datasheet allows only in idle mode, the same way
//! it does after `start_periodic_measurement()`.

#[cfg(all(any(feature = "scd4x", feature = "scd41"), feature = "sync"))]
mod sync {
    use core::convert::Infallible;
    use embedded_hal::delay::DelayNs;
    use embedded_hal::i2c::{ErrorType, I2c, Operation};
    use libscd::error::Error;
    use libscd::synchronous::scd4x::Scd4x;

    /// Accepts every write and counts them
    #[derive(Default)]
    struct BusSpy {
        writes: usize,
    }

    impl ErrorType for BusSpy {
        type Error = Infallible;
    }

    impl I2c for BusSpy {
        fn transaction(
            &mut self,
            _address: u8,
            operations: &mut [Operation<'_>],
        ) -> Result<(), Self::Error> {
            for op in operations {
                match op {
                    Operation::Write(_) => self.writes += 1,
                    Operation::Read(_) => unreachable!("the test does not read"),
                }
            }
            Ok(())
        }
    }

    struct NoDelay;

    impl DelayNs for NoDelay {
        fn delay_ns(&mut self, _ns: u32) {}
    }

    #[test]
    pub fn low_power_periodic_measurement_rejects_idle_only_commands() {
        let mut sensor = Scd4x::new(BusSpy::default(), NoDelay);

        sensor.start_low_power_periodic_measurement().unwrap();
        assert_eq!(Err(Error::NotAllowed), sensor.set_temperature_offset(4.0));

        sensor.stop_periodic_measurement().unwrap();
        assert_eq!(Ok(()), sensor.set_temperature_offset(4.0));

        // start, stop and the second offset; the rejected offset never hits the bus
        assert_eq!(3, sensor.release().writes);
    }
}

#[cfg(all(any(feature = "scd4x", feature = "scd41"), feature = "async"))]
mod asynchronous {
    use core::convert::Infallible;
    use core::future::Future;
    use core::pin::pin;
    use core::task::{Context, Poll, Waker};
    use embedded_hal_async::delay::DelayNs;
    use embedded_hal_async::i2c::{ErrorType, I2c, Operation};
    use libscd::asynchronous::scd4x::Scd4x;
    use libscd::error::Error;

    /// Accepts every write and counts them
    #[derive(Default)]
    struct BusSpy {
        writes: usize,
    }

    impl ErrorType for BusSpy {
        type Error = Infallible;
    }

    impl I2c for BusSpy {
        async fn transaction(
            &mut self,
            _address: u8,
            operations: &mut [Operation<'_>],
        ) -> Result<(), Self::Error> {
            for op in operations {
                match op {
                    Operation::Write(_) => self.writes += 1,
                    Operation::Read(_) => unreachable!("the test does not read"),
                }
            }
            Ok(())
        }
    }

    struct NoDelay;

    impl DelayNs for NoDelay {
        async fn delay_ns(&mut self, _ns: u32) {}
    }

    /// The bus and the delay above never return `Pending`, so a single poll
    /// completes any driver future
    fn ready<F: Future>(future: F) -> F::Output {
        let mut cx = Context::from_waker(Waker::noop());
        match pin!(future).poll(&mut cx) {
            Poll::Ready(output) => output,
            Poll::Pending => unreachable!("the mocks never return Pending"),
        }
    }

    #[test]
    pub fn low_power_periodic_measurement_rejects_idle_only_commands() {
        let mut sensor = Scd4x::new(BusSpy::default(), NoDelay);

        ready(sensor.start_low_power_periodic_measurement()).unwrap();
        assert_eq!(
            Err(Error::NotAllowed),
            ready(sensor.set_temperature_offset(4.0))
        );

        ready(sensor.stop_periodic_measurement()).unwrap();
        assert_eq!(Ok(()), ready(sensor.set_temperature_offset(4.0)));

        // start, stop and the second offset; the rejected offset never hits the bus
        assert_eq!(3, sensor.release().writes);
    }
}
