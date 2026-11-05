#pragma once

#include "rust/cxx.h"

#include <cstdint>
#include <memory>

namespace d2s {

class NativePublisher final {
public:
    NativePublisher(std::uint32_t domain_id, rust::Str topic);
    ~NativePublisher();

    NativePublisher(const NativePublisher&) = delete;
    NativePublisher& operator=(const NativePublisher&) = delete;

    void publish(std::uint32_t index, rust::Str text);
    bool has_readers() const noexcept;

private:
    struct Impl;
    std::unique_ptr<Impl> impl_;
};

class NativeSubscriber final {
public:
    NativeSubscriber(std::uint32_t domain_id, rust::Str topic);
    ~NativeSubscriber();

    NativeSubscriber(const NativeSubscriber&) = delete;
    NativeSubscriber& operator=(const NativeSubscriber&) = delete;

    bool try_take();
    std::uint32_t last_index() const noexcept;
    rust::String last_text() const;

private:
    struct Impl;
    std::unique_ptr<Impl> impl_;
};

std::unique_ptr<NativePublisher> new_publisher(std::uint32_t domain_id, rust::Str topic);
std::unique_ptr<NativeSubscriber> new_subscriber(std::uint32_t domain_id, rust::Str topic);

} // namespace d2s
