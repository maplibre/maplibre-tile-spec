#pragma once

#include <common.hpp>

namespace mlt {

struct BufferStream {
    BufferStream() = delete;
    BufferStream(const BufferStream&) = delete;
    BufferStream(BufferStream&&) = default;
    BufferStream(DataView data_)
        : data(data_), offset(0) {}

    bool available(offset_t size = 1) const { return offset + size < data.size(); }

    DataView::value_type read() {
        if (!available()) {
            throw std::runtime_error("Unexpected end of buffer");
        }
        return data[offset++];
    }

    DataView::value_type peek() const {
        if (!available()) {
            throw std::runtime_error("Unexpected end of buffer");
        }
        return data[offset];
    }

    const auto* getData() const { return data.data(); }
    auto getSize() const { return data.size(); }

    void consume(offset_t count) {
        if (!available(count)) {
            throw std::runtime_error("Unexpected end of buffer");
        }
        offset += count;
    }

private:
    const DataView data;
    offset_t offset;
};

} // namespace mlt
