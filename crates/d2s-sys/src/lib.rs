#![doc = "Internal FFI adapter. Phase 0 intentionally supports a single generated IDL type."]

#[cxx::bridge(namespace = "d2s")]
mod ffi {
    unsafe extern "C++" {
        include!("d2s-sys/native/include/native.hpp");

        type NativePublisher;
        type NativeSubscriber;

        fn new_publisher(domain_id: u32, topic: &str) -> Result<UniquePtr<NativePublisher>>;
        fn publish(self: Pin<&mut NativePublisher>, index: u32, text: &str) -> Result<()>;
        fn has_readers(self: &NativePublisher) -> bool;

        fn new_subscriber(domain_id: u32, topic: &str) -> Result<UniquePtr<NativeSubscriber>>;
        fn try_take(self: Pin<&mut NativeSubscriber>) -> Result<bool>;
        fn last_index(self: &NativeSubscriber) -> u32;
        fn last_text(self: &NativeSubscriber) -> String;
    }
}

/// A single typed sample received from Fast DDS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Received {
    pub index: u32,
    pub text: String,
}

/// Owns its native participant, publisher, and writer.
pub struct Publisher {
    inner: cxx::UniquePtr<ffi::NativePublisher>,
}

impl Publisher {
    pub fn open(domain_id: u32, topic: &str) -> Result<Self, String> {
        let inner = ffi::new_publisher(domain_id, topic).map_err(|error| error.to_string())?;
        Ok(Self { inner })
    }

    pub fn has_readers(&self) -> bool {
        self.inner
            .as_ref()
            .expect("D2S native publisher pointer must be non-null")
            .has_readers()
    }

    pub fn publish(&mut self, index: u32, text: &str) -> Result<(), String> {
        self.inner
            .pin_mut()
            .publish(index, text)
            .map_err(|error| error.to_string())
    }
}

/// Owns its native participant, subscriber, and reader.
pub struct Subscriber {
    inner: cxx::UniquePtr<ffi::NativeSubscriber>,
}

impl Subscriber {
    pub fn open(domain_id: u32, topic: &str) -> Result<Self, String> {
        let inner = ffi::new_subscriber(domain_id, topic).map_err(|error| error.to_string())?;
        Ok(Self { inner })
    }

    pub fn try_receive(&mut self) -> Result<Option<Received>, String> {
        if !self
            .inner
            .pin_mut()
            .try_take()
            .map_err(|error| error.to_string())?
        {
            return Ok(None);
        }

        let native = self
            .inner
            .as_ref()
            .expect("D2S native subscriber pointer must be non-null");
        Ok(Some(Received {
            index: native.last_index(),
            text: native.last_text(),
        }))
    }
}
