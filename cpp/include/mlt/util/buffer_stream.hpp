#pragma once

#include <mlt/common.hpp>
#include <mlt/util/noncopyable.hpp>

#include <cstdint>
#include <stdexcept>

namespace mlt {

struct BufferStream : public util::noncopyable {
    BufferStream() = delete;
    BufferStream(DataView data_) noexcept
        : data(std::move(data_)),
          offset(0) {}
    BufferStream(BufferStream&&) = delete;
    BufferStream& operator=(BufferStream&&) = delete;

    auto getSize() const noexcept { return data.size(); }
    auto getOffset() const noexcept { return offset; }
    auto getRemaining() const noexcept { return data.size() - offset; }
    bool available(std::size_t size = 1) const noexcept { return size <= getRemaining(); }

    template <typename T = std::uint8_t>
    const T* getData() const noexcept {
        return reinterpret_cast<const T*>(data.data());
    }
    template <typename T = std::uint8_t>
    const T* getReadPosition() const noexcept {
        return reinterpret_cast<const T*>(&data[offset]);
    }

    template <typename T = std::uint8_t>
    DataView::value_type read() {
        check(sizeof(T));
        const T* p = getReadPosition<T>();
        consume(sizeof(T));
        return static_cast<DataView::value_type>(*p);
    }

    template <typename T = std::uint8_t>
    DataView::value_type peek() const {
        check(sizeof(T));
        return *getReadPosition<T>();
    }

    void consume(std::uint32_t count) {
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
    std::uint32_t offset;
};

} // namespace mlt
