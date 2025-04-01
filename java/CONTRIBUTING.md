
To build the project run the following command:
````bash
./gradlew build
````

To format the code run the following command:
````bash
./gradlew spotlessApply
````

To execute the benchmarks run the following command:
````bash
./gradlew jmh
````

To build just the CLI tools:
````bash
./gradlew cli
````

To run the tests:
````bash
./gradlew test
````

To run specific tests like the `MltDecoderTest` test:
````bash
./gradlew test --tests com.mlt.decoder.MltDecoderTest
````

View test reports by opening `build/reports/tests/test/index.html`

View test coverage reports by opening `build/reports/jacoco/test/html/index.html`
