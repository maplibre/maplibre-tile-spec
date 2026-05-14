plugins {
    kotlin("jvm") version "2.1.21"
    id("com.vanniktech.maven.publish") version "0.30.0"
}

repositories {
    mavenCentral()
}

dependencies {
    api("net.java.dev.jna:jna:5.17.0")
}

kotlin {
    jvmToolchain(21)
}

// ---------------------------------------------------------------------------
// Maven Central publishing
// ---------------------------------------------------------------------------

tasks.register("validateSemver") {
    doLast {
        val v = project.version.toString()
        if (v == "unspecified") {
            throw GradleException("Version is not set. Please provide a version using -Pversion=<version>")
        }
        if (!v.matches(Regex("""\d+\.\d+\.\d+"""))) {
            throw GradleException("Version '$v' is not a valid semantic version. Expected format: major.minor.patch (e.g., 1.0.0)")
        }
    }
}

mavenPublishing {
    publishToMavenCentral(com.vanniktech.maven.publish.SonatypeHost.CENTRAL_PORTAL, automaticRelease = true)
    signAllPublications()

    coordinates("org.maplibre", "mlt-ffi-kotlin", version.toString())

    pom {
        name.set("MapLibre Tile FFI (Kotlin)")
        description.set("Kotlin/JVM bindings for the Rust MLT encoder via Diplomat FFI")
        url.set("https://github.com/maplibre/maplibre-tile-spec")

        licenses {
            license {
                name.set("MIT License")
                url.set("https://opensource.org/licenses/MIT")
            }
            license {
                name.set("The Apache License, Version 2.0")
                url.set("https://www.apache.org/licenses/LICENSE-2.0.txt")
            }
        }

        developers {
            developer {
                id.set("maplibre")
                name.set("MapLibre contributors")
                url.set("https://github.com/maplibre")
            }
        }

        scm {
            connection.set("scm:git:git://github.com/maplibre/maplibre-tile-spec.git")
            developerConnection.set("scm:git:ssh://github.com:maplibre/maplibre-tile-spec.git")
            url.set("https://github.com/maplibre/maplibre-tile-spec")
        }
    }
}

tasks.named("publishToMavenCentral") {
    dependsOn("validateSemver")
}

// Disable signing for local publishing
tasks.withType<Sign> {
    onlyIf { !gradle.startParameter.taskNames.contains("publishMavenPublicationToMavenLocal") }
}

// ---------------------------------------------------------------------------
// Per-platform classifier JARs containing native libraries
// ---------------------------------------------------------------------------

val platforms = listOf("linux-x86_64", "linux-x86_64-musl", "linux-aarch64", "macos-aarch64", "windows-x86_64")

platforms.forEach { classifier ->
    tasks.register<Jar>("nativeJar-$classifier") {
        archiveClassifier.set(classifier)
        from("${layout.buildDirectory.get()}/natives/$classifier") {
            into("native/$classifier")
        }
    }
}

tasks.register<Jar>("nativeJar-all") {
    archiveClassifier.set("all")
    platforms.forEach { classifier ->
        from("${layout.buildDirectory.get()}/natives/$classifier") {
            into("native/$classifier")
        }
    }
}

afterEvaluate {
    publishing {
        publications.withType<MavenPublication>().configureEach {
            platforms.forEach { classifier ->
                artifact(tasks.named("nativeJar-$classifier")) {
                    this.classifier = classifier
                }
            }
            artifact(tasks.named("nativeJar-all")) {
                this.classifier = "all"
            }
        }
    }
}
