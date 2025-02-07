#pragma once

#include <mlt/metadata/stream.hpp>
#include <mlt/util/buffer_stream.hpp>
#include <mlt/util/noncopyable.hpp>
#include <mlt/util/rle.hpp>
#include <mlt/util/morton.hpp>
#include <mlt/util/vectorized.hpp>
#include <mlt/util/zigzag.hpp>

#include <cstdint>
#include <type_traits>
#include <vector>

namespace mlt::decoder {

class IntegerDecoder : public util::noncopyable {
public:
    using StreamMetadata = metadata::stream::StreamMetadata;
    using MortonEncodedStreamMetadata = metadata::stream::MortonEncodedStreamMetadata;

    IntegerDecoder();
    ~IntegerDecoder() noexcept;

    IntegerDecoder(IntegerDecoder&&) = delete;
    // FastPFOR classes have implicitly-deleted assignment operators.
    // We could create new ones, if really necessary.
    IntegerDecoder& operator=(IntegerDecoder&&) = delete;

    /// Decode a buffer of integers into another, according to the encoding scheme specified by the metadata
    /// @param values input values
    /// @param out output values, automatically resized as necessary
    /// @param metadata stream metadata specifying the encoding details
    template <typename T, typename TTarget = T, bool isSigned = std::is_signed_v<T>>
        requires((std::is_integral_v<T> || std::is_enum_v<T>) &&
                 (std::is_integral_v<TTarget> || std::is_enum_v<TTarget>) && sizeof(T) <= sizeof(TTarget))
    void decodeIntArray(const std::vector<T>& values, std::vector<TTarget>& out, const StreamMetadata& streamMetadata);

    /// Decode an integer stream into the target buffer
    /// @param tileData source data
    /// @param out output data, automatically resized
    /// @param metadata stream metadata specifying the encoding details
    /// @details Uses an internal buffer for intermediate values
    template <typename TDecode,
              typename TInt = TDecode,
              typename TTarget = TDecode,
              bool isSigned = std::is_signed_v<TDecode>>
    void decodeIntStream(BufferStream& tileData, std::vector<TTarget>& out, const StreamMetadata& metadata);

    /// Decode an integer stream into the target buffer
    /// @param tileData source data
    /// @param buffer storage for intermediate values, automatically resized
    /// @param out output data, automatically resized
    /// @param metadata stream metadata specifying the encoding details
    template <typename TDecode,
              typename TInt = TDecode,
              typename TTarget = TInt,
              bool isSigned = std::is_signed_v<TDecode>>
    void decodeIntStream(BufferStream& tileData,
                         std::vector<TInt>& buffer,
                         std::vector<TTarget>& out,
                         const StreamMetadata& metadata);

    template <typename TDecode, typename TInt = TDecode, typename TTarget = TInt, bool Delta = true>
    void decodeMortonStream(BufferStream& tileData,
                            std::vector<TTarget>& out,
                            const MortonEncodedStreamMetadata& metadata);

    template <typename TDecode, typename TInt = TDecode, typename TTarget = TInt, bool Delta = true>
    void decodeMortonStream(BufferStream& tileData,
                            std::vector<TInt>& buffer,
                            std::vector<TTarget>& out,
                            const MortonEncodedStreamMetadata& metadata);

private:
    struct Impl;
    std::unique_ptr<Impl> impl;

    template <typename T>
    std::vector<T>& getTempBuffer() noexcept;

    template <typename TDecode, typename TTarget = TDecode, bool isSigned = std::is_signed_v<TDecode>>
        requires(std::is_integral_v<TDecode> && (std::is_integral_v<TTarget> || std::is_enum_v<TTarget>))
    void decodeStream(BufferStream& tileData, std::vector<TTarget>& out, const StreamMetadata& metadata);

    template <typename T>
    static void decodeRLE(const std::vector<T>& values, std::vector<T>& out, const count_t numRuns);

    template <typename T>
    static void decodeDeltaRLE(const std::vector<T>& values, std::vector<T>& out, const count_t numRuns);

    /// Decode zigzag-delta values
    /// @param values input values
    /// @param out output values, must be the same size as the input
    /// @note Input and output may reference the same vector
    template <typename T, typename TTarget>
        requires(std::is_integral_v<underlying_type_t<T>> && std::is_integral_v<underlying_type_t<TTarget>> &&
                 sizeof(T) <= sizeof(TTarget))
    static void decodeZigZagDelta(const std::vector<T>& values, std::vector<TTarget>& out) noexcept;

    /// Decode standard or delta Morton codes
    /// @param data input data
    /// @param out output data, must be 2x the input size
    template <typename TDecode, typename TTarget, bool delta>
        requires(std::is_integral_v<TDecode> && (std::is_integral_v<TTarget> || std::is_enum_v<TTarget>))
    static void decodeMortonCodes(const std::vector<TDecode>& data,
                                  std::vector<TTarget>& out,
                                  int numBits,
                                  int coordinateShift) noexcept;

    std::uint32_t decodeFastPfor(BufferStream& buffer,
                                 std::uint32_t* const result,
                                 const std::size_t numValues,
                                 const std::size_t byteLength);
};
} // namespace mlt::decoder
