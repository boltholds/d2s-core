#include "d2s-sys/native/include/native.hpp"

#include "D2sMessage.hpp"
#include "D2sMessagePubSubTypes.hpp"

#include <fastdds/dds/core/policy/QosPolicies.hpp>
#include <fastdds/dds/core/status/PublicationMatchedStatus.hpp>
#include <fastdds/dds/core/status/StatusMask.hpp>
#include <fastdds/dds/domain/DomainParticipant.hpp>
#include <fastdds/dds/domain/DomainParticipantFactory.hpp>
#include <fastdds/dds/publisher/DataWriter.hpp>
#include <fastdds/dds/publisher/DataWriterListener.hpp>
#include <fastdds/dds/publisher/qos/DataWriterQos.hpp>
#include <fastdds/dds/publisher/qos/PublisherQos.hpp>
#include <fastdds/dds/publisher/Publisher.hpp>
#include <fastdds/dds/subscriber/DataReader.hpp>
#include <fastdds/dds/subscriber/SampleInfo.hpp>
#include <fastdds/dds/subscriber/Subscriber.hpp>
#include <fastdds/dds/subscriber/qos/DataReaderQos.hpp>
#include <fastdds/dds/subscriber/qos/SubscriberQos.hpp>
#include <fastdds/dds/topic/Topic.hpp>
#include <fastdds/dds/topic/TypeSupport.hpp>

#include <atomic>
#include <cstdint>
#include <memory>
#include <stdexcept>
#include <string>

namespace d2s {
namespace dds = eprosima::fastdds::dds;
namespace {

std::string to_std_string(rust::Str value)
{
    return std::string(value.data(), value.size());
}

// All Fast DDS entities belong to a single DomainParticipant.
// Its destructor deletes children BEFORE the participant itself.
class ParticipantContext final {
public:
    ParticipantContext(std::uint32_t domain_id, const std::string& topic_name)
        : type_(new D2sMessagePubSubType())
    {
        if (topic_name.empty())
        {
            throw std::invalid_argument("DDS topic must not be empty");
        }

        try
        {
            auto* factory = dds::DomainParticipantFactory::get_instance();
            participant_ = factory->create_participant(domain_id, dds::PARTICIPANT_QOS_DEFAULT);
            if (participant_ == nullptr)
            {
                throw std::runtime_error("Fast DDS failed to create DomainParticipant");
            }

            if (type_.register_type(participant_) != dds::RETCODE_OK)
            {
                throw std::runtime_error("Fast DDS failed to register D2sMessage type");
            }

            topic_ = participant_->create_topic(
                topic_name,
                type_.get_type_name(),
                dds::TOPIC_QOS_DEFAULT);

            if (topic_ == nullptr)
            {
                throw std::runtime_error("Fast DDS failed to create Topic");
            }
        }
        catch (...)
        {
            close();
            throw;
        }
    }

    ParticipantContext(const ParticipantContext&) = delete;
    ParticipantContext& operator=(const ParticipantContext&) = delete;

    ~ParticipantContext()
    {
        close();
    }

    dds::DomainParticipant* participant() const noexcept
    {
        return participant_;
    }

    dds::Topic* topic() const noexcept
    {
        return topic_;
    }

private:
    void close() noexcept
    {
        if (participant_ != nullptr)
        {
            participant_->delete_contained_entities();
            dds::DomainParticipantFactory::get_instance()->delete_participant(participant_);
            participant_ = nullptr;
            topic_ = nullptr;
        }
    }

    dds::TypeSupport type_;
    dds::DomainParticipant* participant_{nullptr};
    dds::Topic* topic_{nullptr};
};

class MatchListener final : public dds::DataWriterListener {
public:
    void on_publication_matched(
        dds::DataWriter*,
        const dds::PublicationMatchedStatus& status) override
    {
        matched.store(status.current_count, std::memory_order_relaxed);
    }

    std::atomic<std::int32_t> matched{0};
};

} // namespace

struct NativePublisher::Impl {
    // Important: the listener outlives the participant's writer.
    MatchListener listener;
    ParticipantContext context;
    dds::Publisher* publisher{nullptr};
    dds::DataWriter* writer{nullptr};

    Impl(std::uint32_t domain, const std::string& topic)
        : context(domain, topic)
    {
        publisher = context.participant()->create_publisher(dds::PUBLISHER_QOS_DEFAULT);
        if (publisher == nullptr)
        {
            throw std::runtime_error("Fast DDS failed to create Publisher");
        }

        dds::DataWriterQos writer_qos = dds::DATAWRITER_QOS_DEFAULT;
        publisher->get_default_datawriter_qos(writer_qos);
        writer_qos.reliability().kind = dds::RELIABLE_RELIABILITY_QOS;
        writer_qos.history().kind = dds::KEEP_LAST_HISTORY_QOS;
        writer_qos.history().depth = 16;

        writer = publisher->create_datawriter(
            context.topic(),
            writer_qos,
            &listener,
            dds::StatusMask::all());

        if (writer == nullptr)
        {
            throw std::runtime_error("Fast DDS failed to create DataWriter");
        }
    }
};

NativePublisher::NativePublisher(std::uint32_t domain_id, rust::Str topic)
    : impl_(std::make_unique<Impl>(domain_id, to_std_string(topic)))
{
}

NativePublisher::~NativePublisher() = default;

bool NativePublisher::has_readers() const noexcept
{
    return impl_->listener.matched.load(std::memory_order_relaxed) > 0;
}

void NativePublisher::publish(std::uint32_t index, rust::Str text)
{
    D2sMessage message;
    message.index(index);
    message.text(to_std_string(text));

    if (impl_->writer->write(&message) != dds::RETCODE_OK)
    {
        throw std::runtime_error("Fast DDS DataWriter::write failed");
    }
}

struct NativeSubscriber::Impl {
    ParticipantContext context;
    dds::Subscriber* subscriber{nullptr};
    dds::DataReader* reader{nullptr};
    std::uint32_t last_index{0};
    std::string last_text;

    Impl(std::uint32_t domain, const std::string& topic)
        : context(domain, topic)
    {
        subscriber = context.participant()->create_subscriber(dds::SUBSCRIBER_QOS_DEFAULT);
        if (subscriber == nullptr)
        {
            throw std::runtime_error("Fast DDS failed to create Subscriber");
        }

        dds::DataReaderQos reader_qos = dds::DATAREADER_QOS_DEFAULT;
        subscriber->get_default_datareader_qos(reader_qos);
        reader_qos.reliability().kind = dds::RELIABLE_RELIABILITY_QOS;
        reader_qos.history().kind = dds::KEEP_LAST_HISTORY_QOS;
        reader_qos.history().depth = 16;

        reader = subscriber->create_datareader(
            context.topic(),
            reader_qos,
            nullptr,
            dds::StatusMask::none());

        if (reader == nullptr)
        {
            throw std::runtime_error("Fast DDS failed to create DataReader");
        }
    }
};

NativeSubscriber::NativeSubscriber(std::uint32_t domain_id, rust::Str topic)
    : impl_(std::make_unique<Impl>(domain_id, to_std_string(topic)))
{
}

NativeSubscriber::~NativeSubscriber() = default;

bool NativeSubscriber::try_take()
{
    D2sMessage message;
    dds::SampleInfo info;

    while (true)
    {
        auto status = impl_->reader->take_next_sample(&message, &info);
        if (status == dds::RETCODE_NO_DATA)
        {
            return false;
        }
        if (status != dds::RETCODE_OK)
        {
            throw std::runtime_error("Fast DDS DataReader::take_next_sample failed");
        }
        if (info.valid_data)
        {
            impl_->last_index = message.index();
            impl_->last_text = message.text();
            return true;
        }
    }
}

std::uint32_t NativeSubscriber::last_index() const noexcept
{
    return impl_->last_index;
}

rust::String NativeSubscriber::last_text() const
{
    return rust::String::lossy(impl_->last_text);
}

std::unique_ptr<NativePublisher> new_publisher(std::uint32_t domain_id, rust::Str topic)
{
    return std::make_unique<NativePublisher>(domain_id, topic);
}

std::unique_ptr<NativeSubscriber> new_subscriber(std::uint32_t domain_id, rust::Str topic)
{
    return std::make_unique<NativeSubscriber>(domain_id, topic);
}

} // namespace d2s
