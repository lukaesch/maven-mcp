# maven-mcp

A Model Context Protocol (MCP) server that connects AI assistants to Maven Central, enabling dependency version lookup, update detection, and project health analysis.

Built with Rust for fast startup and minimal resource usage.

## Features

- **Version Lookup**: Get latest stable, RC, beta, and alpha versions for any Maven dependency
- **Version Verification**: Check if specific versions exist on Maven Central
- **Update Detection**: Compare your current versions against latest and classify updates (major/minor/patch)
- **Bulk Analysis**: Check multiple dependencies at once for efficient project analysis
- **Age Analysis**: Classify dependencies as current, fresh, aging, stale, or outdated
- **Health Scoring**: Get an overall health score (A-F grade) for your project's dependencies

## Installation

### Claude Desktop (MCPB Bundle)

Download `maven-mcp.mcpb` from the [releases page](https://github.com/lukaesch/maven-mcp/releases) and double-click to install, or drag it into Claude Desktop.

### From Source

```bash
git clone https://github.com/lukaesch/maven-mcp.git
cd maven-mcp
cargo build --release
```

## Tools

### `get_latest_version`

Get the latest version of a Maven dependency with stability classification.

**Parameters:**
- `dependency` (required): Maven coordinate like `org.springframework:spring-core`
- `prefer_stable` (optional, default: true): Prioritize stable versions

**Example prompt:**
```
What's the latest version of spring-boot-starter-web?
```

### `check_version_exists`

Verify if a specific version exists on Maven Central.

**Parameters:**
- `dependency` (required): Maven coordinate with version like `org.springframework:spring-core:6.1.0`

**Example prompt:**
```
Does spring-core version 6.1.0 exist?
```

### `compare_versions`

Compare your current version against the latest and determine update type.

**Parameters:**
- `dependency` (required): Maven coordinate with version like `org.springframework:spring-core:5.3.0`
- `stable_only` (optional, default: true): Only suggest stable version upgrades

**Example prompt:**
```
How outdated is my spring-core 5.3.0 dependency?
```

### `check_multiple_dependencies`

Bulk check multiple Maven dependencies for available updates.

**Parameters:**
- `dependencies` (required): List of Maven coordinates
- `stable_only` (optional, default: true): Only suggest stable version upgrades

**Example prompt:**
```
Check these dependencies for updates:
- org.springframework.boot:spring-boot-starter-web:2.7.0
- com.fasterxml.jackson.core:jackson-databind:2.14.0
- org.apache.commons:commons-lang3:3.12.0
```

### `analyze_dependency_age`

Analyze how outdated a dependency is.

**Parameters:**
- `dependency` (required): Maven coordinate with version

**Example prompt:**
```
How stale is my hibernate-core 5.6.0 dependency?
```

### `analyze_project_health`

Comprehensive health analysis of all project dependencies.

**Parameters:**
- `dependencies` (required): List of Maven coordinates with versions

**Example prompt:**
```
Analyze the health of my project dependencies:
[paste your pom.xml or build.gradle dependencies]
```

## Building from Source

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test
```

## Creating MCPB Bundle

```bash
mkdir -p bundle/server
cp manifest.json bundle/
cp target/release/maven-mcp bundle/server/
cd bundle && zip -r ../maven-mcp.mcpb .
```

## Architecture

```
src/
├── main.rs          # Entry point, MCP server setup
├── lib.rs           # Library exports
├── models/          # Data structures
│   ├── coordinate.rs   # Maven coordinate parsing
│   └── version.rs      # Version classification & comparison
├── maven/           # Maven Central client
│   ├── client.rs       # HTTP client with caching
│   └── metadata.rs     # maven-metadata.xml parsing
└── tools/           # MCP tools
    ├── service.rs      # Tool implementations
    └── responses.rs    # Response types
```

## License

MIT
