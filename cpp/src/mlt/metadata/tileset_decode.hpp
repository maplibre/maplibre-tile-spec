#pragma once

#include <mlt/metadata/tileset.hpp>
#include <mlt/util/buffer_stream.hpp>
#include <mlt/util/memory_budget.hpp>

namespace mlt::metadata::tileset {

/// Decode a feature table, charging the decoded columns to `budget`
/// @throws std::runtime_error if the data is invalid or the columns exceed the budget
FeatureTable decodeFeatureTable(BufferStream&, util::MemoryBudget&);

} // namespace mlt::metadata::tileset
