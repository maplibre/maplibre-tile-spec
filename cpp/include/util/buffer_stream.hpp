#pragma once

#include <common.hpp>

#include <stdexcept>

namespace mlt {

struct BufferStream {
    BufferStream() = delete;
    BufferStream(const BufferStream&) = delete;
    BufferStream(BufferStream&&) = default;
    BufferStream(DataView data_)
        : data(data_),
          offset(0) {}

    auto getSize() const { return data.size(); }
    auto getOffset() const { return offset; }
    auto getRemaining() const { return data.size() - offset; }
    bool available(std::size_t size = 1) const { return size <= getRemaining(); }

    template <typename T = std::uint8_t>
    const T* getData() const {
        return reinterpret_cast<const T*>(data.data());
    }
    template <typename T = std::uint8_t>
    const T* getReadPosition() const {
        return reinterpret_cast<const T*>(&data[offset]);
    }

    template <typename T = std::uint8_t>
    DataView::value_type read() {
        check(sizeof(T));
        const T* p = getReadPosition<T>();
        consume(sizeof(T));
        return *p;
    }

    template <typename T = std::uint8_t>
    DataView::value_type peek() const {
        check(sizeof(T));
        return *getReadPosition<T>();
    }

    void consume(offset_t count) {
        check(count);
        offset += count;
    }

private:
    void check(std::size_t count) const {
        if (!available(count)) {
            throw std::runtime_error("Unexpected end of buffer");
        }
    }

    const DataView data;
    offset_t offset;
};

} // namespace mlt
