```rockuml tldr
@startuml
title rockuml as your editor's diagram server
actor You
participant "VS Code or\nIntelliJ plugin" as Plugin
participant "rockuml\n--http-server" as Server

You -> Plugin : edit diagram.puml
Plugin -> Server : GET /svg/<code>
activate Server
Server -> Server : decode, render
Server --> Plugin : image/svg+xml
deactivate Server
Plugin --> You : live preview
@enduml
```

Editor plugins for PlantUML draw their previews either with a local Java installation or by calling a PlantUML server. `rockuml --http-server` is such a server, compatible with PlantUML's own, so the plugins get rockuml's speed without Java and without sending your diagrams over the internet.

## Starting the server

```bash
rockuml --http-server                      # port 8080, on every network interface
rockuml --http-server:9000                 # port 9000
rockuml --http-server:9000:127.0.0.1       # port 9000, this machine only
rockuml --http-server:9000:127.0.0.1:stop  # also lets GET /stopserver stop it
```

Listening on `127.0.0.1` is the safe choice on shared networks: only programs on your machine can reach the server.

## Editor setup

### VS Code

In the [PlantUML extension](https://marketplace.visualstudio.com/items?itemName=jebbs.plantuml)'s settings:

```json
{
  "plantuml.render": "PlantUMLServer",
  "plantuml.server": "http://localhost:8080"
}
```

### JetBrains IDEs

In the settings of the PlantUML Integration plugin, switch to remote rendering and enter `http://localhost:8080` as the server URL.

### Anything else

Any tool that can use a PlantUML server URL works the same way: point it at `http://localhost:8080`.

## The endpoints

| Request | Answer |
|---|---|
| `GET /svg/<code>`, `GET /plantuml/svg/<code>` | the diagram as SVG |
| `GET /png/<code>`, `GET /plantuml/png/<code>` | the diagram as PNG |
| `POST /render` with `{"source": "…", "options": ["-tsvg"]}` | the diagram, rendered with command line options |
| `GET /serverinfo` | the server's version |
| `GET /stopserver` | stops the server, if started with `:stop` |

`<code>` is the diagram source in PlantUML's URL encoding; `rockuml --encode-url` makes one from a file. The status codes and the headers that carry an image's size and errors are PlantUML's, so plugins can't tell the difference.

```bash
curl http://localhost:8080/svg/SyfFKj2rKt3CoKnELR1Io4ZDoSa70000 > hello.svg
curl -X POST http://localhost:8080/render \
  -H "Content-Type: application/json" \
  -d '{"source": "@startuml\nA -> B\n@enduml", "options": ["-tsvg"]}'
```
