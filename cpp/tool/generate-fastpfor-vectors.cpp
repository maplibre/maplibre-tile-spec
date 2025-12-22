/**
 * Tool to generate FastPFOR test vectors for cross-language validation.
 * 
 * Generates compressed vectors for specific bitwidths to test:
 * - Unrolled bitwidths: 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 16
 * - Generic fallback bitwidths: 13, 14, 15
 * 
 * Output: C++ array literals that can be pasted into test_fastpfor.cpp
 * 
 * Usage: 
 *   ./generate-fastpfor-vectors                                    # defaults
 *   ./generate-fastpfor-vectors --bitwidths 1,4,8,16               # specific bitwidths
 *   ./generate-fastpfor-vectors --count 66000                      # multi-page
 *   ./generate-fastpfor-vectors --pattern dense                    # force bestB=bw
 *   ./generate-fastpfor-vectors --bitwidths 4,8 --count 1024 --pattern mixed
 */

#include <cstdint>
#include <cstdio>
#include <cstring>
#include <iomanip>
#include <iostream>
#include <sstream>
#include <vector>

#include <fastpfor.h>
#include <compositecodec.h>
#include <variablebyte.h>

using namespace FastPForLib;

constexpr size_t BLOCK_SIZE = 256;
constexpr size_t PAGE_SIZE = 65536;

enum class Pattern { DENSE, MIXED, EXCEPTIONS_PAYLOAD };

// Parse comma-separated list of integers
std::vector<int> parseBitwidths(const char* arg) {
    std::vector<int> result;
    std::istringstream ss(arg);
    std::string token;
    while (std::getline(ss, token, ',')) {
        int bw = std::stoi(token);
        if (bw >= 0 && bw <= 32) {
            result.push_back(bw);
        }
    }
    return result;
}

// Generate values that fit in exactly `bitwidth` bits
std::vector<uint32_t> generateValues(int bitwidth, size_t count, Pattern pattern) {
    std::vector<uint32_t> values(count);
    
    if (bitwidth == 0) {
        return values; // all zeros
    }
    
    uint32_t maxVal = (bitwidth == 32) ? 0xFFFFFFFF : ((1u << bitwidth) - 1);
    
    switch (pattern) {
        case Pattern::DENSE:
            // Forces bestB == bitwidth (no exceptions possible)
            for (size_t i = 0; i < count; i++) {
                values[i] = (i & 1) ? maxVal : (maxVal > 0 ? maxVal - 1 : 0);
            }
            break;
            
        case Pattern::MIXED:
            // Mix of values - may trigger exceptions
            for (size_t i = 0; i < count; i++) {
                if (i % 3 == 0) {
                    values[i] = maxVal;
                } else if (i % 3 == 1) {
                    values[i] = i % (maxVal + 1);
                } else {
                    values[i] = (i * 12345) % (maxVal + 1);
                }
            }
            break;
            
        case Pattern::EXCEPTIONS_PAYLOAD:
            // Majority in 10 bits, exceptions at ~20 bits (index > 1)
            for (size_t i = 0; i < count; i++) {
                values[i] = i & 1023; // 10 bits
            }
            // Add exceptions that require payload
            if (count > 5) values[5] = (1u << 19) + 123;
            if (count > 123) values[123] = (1u << 19) + 4567;
            if (count > 400) values[400] = (1u << 19) + 9999;
            break;
    }
    
    return values;
}

// Encode using CompositeCodec(FastPFor, VariableByte) - same as JavaFastPFOR
std::vector<uint32_t> encodeFastPfor(const std::vector<uint32_t>& values) {
    CompositeCodec<FastPFor<8>, VariableByte> codec;
    
    std::vector<uint32_t> compressed(values.size() + 1024);
    size_t compressedSize = compressed.size();
    
    codec.encodeArray(values.data(), values.size(), compressed.data(), compressedSize);
    compressed.resize(compressedSize);
    
    return compressed;
}

void printCppArray(const std::string& name, const std::vector<uint32_t>& data) {
    std::cout << "const std::uint32_t " << name << "[] = { ";
    for (size_t i = 0; i < data.size(); i++) {
        if (i > 0) std::cout << ", ";
        // Hex with padding for readability and diff-friendly output
        std::cout << "0x" << std::setw(8) << std::setfill('0') << std::hex << data[i] << "u" << std::dec;
    }
    std::cout << " };" << std::endl;
}

void printUsage(const char* progname) {
    std::cerr << "Usage: " << progname << " [OPTIONS]" << std::endl;
    std::cerr << "Options:" << std::endl;
    std::cerr << "  --bitwidths 1,4,8,16   Comma-separated list of bitwidths (default: 4,8,12,13,16)" << std::endl;
    std::cerr << "  --count N              Number of values to generate (default: 512)" << std::endl;
    std::cerr << "  --pattern P            Pattern: dense, mixed, exceptions_payload (default: dense)" << std::endl;
    std::cerr << "  --help                 Show this message" << std::endl;
    std::cerr << std::endl;
    std::cerr << "Block size notes:" << std::endl;
    std::cerr << "  count < 256            -> VariableByte only (no FastPFOR)" << std::endl;
    std::cerr << "  count % 256 != 0       -> FastPFOR + VariableByte remainder" << std::endl;
    std::cerr << "  count > 65536          -> Multi-page encoding" << std::endl;
}

void printBlockSizeInfo(size_t count) {
    if (count < BLOCK_SIZE) {
        std::cerr << "// Note: count=" << count << " < 256 -> VariableByte only (no FastPFOR blocks)" << std::endl;
    } else if (count % BLOCK_SIZE != 0) {
        size_t aligned = (count / BLOCK_SIZE) * BLOCK_SIZE;
        size_t remainder = count - aligned;
        std::cerr << "// Note: count=" << count << " -> " << aligned << " FastPFOR + " << remainder << " VariableByte remainder" << std::endl;
    }
    if (count > PAGE_SIZE) {
        std::cerr << "// Note: count=" << count << " > 65536 -> Multi-page encoding" << std::endl;
    }
}

int main(int argc, char* argv[]) {
    // Default values
    std::vector<int> bitwidths = {4, 8, 12, 13, 16};
    size_t count = 512;
    Pattern pattern = Pattern::DENSE;
    
    // Parse command line arguments
    for (int i = 1; i < argc; i++) {
        if (std::strcmp(argv[i], "--bitwidths") == 0 && i + 1 < argc) {
            bitwidths = parseBitwidths(argv[++i]);
        } else if (std::strcmp(argv[i], "--count") == 0 && i + 1 < argc) {
            count = std::stoul(argv[++i]);
        } else if (std::strcmp(argv[i], "--pattern") == 0 && i + 1 < argc) {
            std::string p = argv[++i];
            if (p == "dense") pattern = Pattern::DENSE;
            else if (p == "mixed") pattern = Pattern::MIXED;
            else if (p == "exceptions_payload") pattern = Pattern::EXCEPTIONS_PAYLOAD;
        } else if (std::strcmp(argv[i], "--help") == 0) {
            printUsage(argv[0]);
            return 0;
        }
    }
    
    std::cout << "// Cross-language validation vectors for FastPFOR" << std::endl;
    std::cout << "// Generated by generate-fastpfor-vectors tool" << std::endl;
    std::cout << "// Pattern: " << (pattern == Pattern::DENSE ? "dense" : pattern == Pattern::MIXED ? "mixed" : "exceptions_payload") << std::endl;
    std::cout << std::endl;
    
    printBlockSizeInfo(count);
    
    for (int bw : bitwidths) {
        auto uncompressed = generateValues(bw, count, pattern);
        auto compressed = encodeFastPfor(uncompressed);
        
        std::string suffix = std::to_string(bw) + "bit";
        
        std::cout << "// Bitwidth " << bw << " test vectors (" << uncompressed.size() << " values)" << std::endl;
        printCppArray("uncompressed_" + suffix, uncompressed);
        printCppArray("compressed_" + suffix, compressed);
        std::cout << std::endl;
    }
    
    return 0;
}
