# Exclude vendor/third-party code from coverage reports
set(CTEST_CUSTOM_COVERAGE_EXCLUDE
    ${CTEST_CUSTOM_COVERAGE_EXCLUDE}
    ".*/vendor/.*"
    ".*/build/.*/_deps/.*"
    ".*/test/.*"
)
