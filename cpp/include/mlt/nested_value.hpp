#pragma once

#include <cstdint>
#include <memory>
#include <string>
#include <utility>
#include <variant>
#include <vector>

namespace mlt {

class NestedValue;

struct NestedObjectEntry;
using NestedArray = std::vector<NestedValue>;
using NestedObject = std::vector<NestedObjectEntry>;

/// A nested property value, either a scalar or a tree of arrays and objects
class NestedValue {
private:
    using Storage = std::variant<std::nullptr_t,
                                 bool,
                                 std::int64_t,
                                 std::uint64_t,
                                 float,
                                 double,
                                 std::string,
                                 std::unique_ptr<NestedArray>,
                                 std::unique_ptr<NestedObject>>;

public:
    NestedValue()
        : data(nullptr) {}
    explicit NestedValue(std::nullptr_t)
        : data(nullptr) {}
    explicit NestedValue(bool v)
        : data(v) {}
    explicit NestedValue(std::int64_t v)
        : data(v) {}
    explicit NestedValue(std::uint64_t v)
        : data(v) {}
    explicit NestedValue(float v)
        : data(v) {}
    explicit NestedValue(double v)
        : data(v) {}
    explicit NestedValue(std::string v)
        : data(std::move(v)) {}

    explicit NestedValue(NestedArray v);
    explicit NestedValue(NestedObject v);

    ~NestedValue();

    NestedValue(NestedValue&& other) noexcept;
    NestedValue& operator=(NestedValue&& other) noexcept;

    NestedValue(const NestedValue& other);
    NestedValue& operator=(const NestedValue& other);

    bool isNull() const { return std::holds_alternative<std::nullptr_t>(data); }
    bool isBool() const { return std::holds_alternative<bool>(data); }
    bool isInt64() const { return std::holds_alternative<std::int64_t>(data); }
    bool isUint64() const { return std::holds_alternative<std::uint64_t>(data); }
    bool isFloat() const { return std::holds_alternative<float>(data); }
    bool isDouble() const { return std::holds_alternative<double>(data); }
    bool isString() const { return std::holds_alternative<std::string>(data); }
    bool isArray() const { return std::holds_alternative<std::unique_ptr<NestedArray>>(data); }
    bool isObject() const { return std::holds_alternative<std::unique_ptr<NestedObject>>(data); }

    bool getBool() const { return std::get<bool>(data); }
    std::int64_t getInt64() const { return std::get<std::int64_t>(data); }
    std::uint64_t getUint64() const { return std::get<std::uint64_t>(data); }
    float getFloat() const { return std::get<float>(data); }
    double getDouble() const { return std::get<double>(data); }
    const std::string& getString() const { return std::get<std::string>(data); }
    const NestedArray& getArray() const { return *std::get<std::unique_ptr<NestedArray>>(data); }
    const NestedObject& getObject() const { return *std::get<std::unique_ptr<NestedObject>>(data); }

private:
    Storage data;
    static Storage cloneStorage(const Storage& src);
};

struct NestedObjectEntry {
    std::string key;
    NestedValue value;
};

inline NestedValue::NestedValue(NestedArray v)
    : data(std::make_unique<NestedArray>(std::move(v))) {}
inline NestedValue::NestedValue(NestedObject v)
    : data(std::make_unique<NestedObject>(std::move(v))) {}

inline NestedValue::~NestedValue() = default;

inline NestedValue::NestedValue(NestedValue&& other) noexcept = default;
inline NestedValue& NestedValue::operator=(NestedValue&& other) noexcept = default;

inline NestedValue::NestedValue(const NestedValue& other)
    : data(cloneStorage(other.data)) {}

inline NestedValue& NestedValue::operator=(const NestedValue& other) {
    if (this != &other) {
        data = cloneStorage(other.data);
    }
    return *this;
}

inline NestedValue::Storage NestedValue::cloneStorage(const Storage& src) {
    return std::visit(
        [](const auto& val) -> Storage {
            using T = std::decay_t<decltype(val)>;
            if constexpr (std::is_same_v<T, std::unique_ptr<NestedArray>>) {
                return std::make_unique<NestedArray>(*val);
            } else if constexpr (std::is_same_v<T, std::unique_ptr<NestedObject>>) {
                return std::make_unique<NestedObject>(*val);
            } else {
                return val;
            }
        },
        src);
}

} // namespace mlt
