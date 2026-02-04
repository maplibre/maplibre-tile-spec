#pragma once

#include <mlt/metadata/stream.hpp>

#include <cstdint>
#include <cstring>
#include <span>
#include <vector>

namespace mlt::encoder {

class FloatEncoder {
public:
    using StreamMetadata = metadata::stream::StreamMetadata;

    static std::vector<std::uint8_t> encodeFloatStream(std::span<const float> values) {
        const auto byteLength = static_cast<std::uint32_t>(values.size() * sizeof(float));

        auto metadata = StreamMetadata(
                            metadata::stream::PhysicalStreamType::DATA, std::nullopt,
                            metadata::stream::LogicalLevelTechnique::NONE,
                            metadata::stream::LogicalLevelTechnique::NONE,
                            metadata::stream::PhysicalLevelTechnique::NONE,
                            static_cast<std::uint32_t>(values.size()),
                            byteLength)
                            .encode();

        std::vector<std::uint8_t> result;
        result.reserve(metadata.size() + byteLength);
        result.insert(result.end(), metadata.begin(), metadata.end());

        const auto* bytes = reinterpret_cast<const std::uint8_t*>(values.data());
        result.insert(result.end(), bytes, bytes + byteLength);
        return result;
    }
};

} // namespace mlt::encoder
