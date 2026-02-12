//! Resource limits and metadata types.

/// Resource limits for decode/encode operations.
///
/// Used to prevent DoS attacks and resource exhaustion. All limits are optional.
#[derive(Clone, Debug, Default)]
pub struct Limits {
    /// Maximum image width in pixels.
    pub max_width: Option<u64>,
    /// Maximum image height in pixels.
    pub max_height: Option<u64>,
    /// Maximum total pixels (width × height).
    pub max_pixels: Option<u64>,
    /// Maximum memory allocation in bytes.
    pub max_memory_bytes: Option<u64>,
}

impl Limits {
    /// Create a new Limits with no restrictions.
    pub fn none() -> Self {
        Self::default()
    }

    /// Check if dimensions are within limits.
    ///
    /// Returns `Err` with a description if any limit is exceeded.
    pub fn check_dimensions(&self, width: u64, height: u64) -> Result<(), &'static str> {
        if let Some(max_width) = self.max_width {
            if width > max_width {
                return Err("width exceeds limit");
            }
        }

        if let Some(max_height) = self.max_height {
            if height > max_height {
                return Err("height exceeds limit");
            }
        }

        if let Some(max_pixels) = self.max_pixels {
            let pixels = width.saturating_mul(height);
            if pixels > max_pixels {
                return Err("pixel count exceeds limit");
            }
        }

        Ok(())
    }

    /// Check if a memory allocation is within limits.
    pub fn check_memory(&self, bytes: u64) -> Result<(), &'static str> {
        if let Some(max_memory) = self.max_memory_bytes {
            if bytes > max_memory {
                return Err("memory allocation exceeds limit");
            }
        }
        Ok(())
    }
}

impl Limits {
    /// Validate dimensions and estimated memory against limits, returning CodecError on violation.
    pub(crate) fn validate(
        &self,
        width: u32,
        height: u32,
        bytes_per_pixel: u32,
    ) -> Result<(), crate::CodecError> {
        self.check_dimensions(width as u64, height as u64)
            .map_err(|msg| crate::CodecError::LimitExceeded(msg.into()))?;

        let estimated_bytes = (width as u64)
            .saturating_mul(height as u64)
            .saturating_mul(bytes_per_pixel as u64);
        self.check_memory(estimated_bytes)
            .map_err(|msg| crate::CodecError::LimitExceeded(msg.into()))?;

        Ok(())
    }
}

/// Get a `&dyn Stop` reference, defaulting to `Unstoppable` if `None`.
pub(crate) fn stop_or_default(stop: Option<&dyn Stop>) -> &dyn Stop {
    stop.unwrap_or(&enough::Unstoppable)
}

/// Re-export `Stop` for cooperative cancellation.
///
/// Codecs periodically call `stop.check()` and return `CodecError::Cancelled`
/// if the operation should be cancelled. Use `enough::Unstoppable` when you
/// don't need cancellation (zero-cost).
pub use zencodec_types::Stop;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_none() {
        let limits = Limits::none();
        assert!(limits.check_dimensions(u64::MAX, u64::MAX).is_ok());
        assert!(limits.check_memory(u64::MAX).is_ok());
    }

    #[test]
    fn limits_dimensions() {
        let limits = Limits {
            max_width: Some(1000),
            max_height: Some(1000),
            max_pixels: Some(500_000),
            ..Default::default()
        };

        assert!(limits.check_dimensions(1000, 1000).is_err()); // 1M pixels > 500k
        assert!(limits.check_dimensions(500, 500).is_ok()); // 250k pixels
        assert!(limits.check_dimensions(2000, 500).is_err()); // width > 1000
    }

    #[test]
    fn limits_memory() {
        let limits = Limits {
            max_memory_bytes: Some(1_000_000),
            ..Default::default()
        };

        assert!(limits.check_memory(500_000).is_ok());
        assert!(limits.check_memory(2_000_000).is_err());
    }
}
