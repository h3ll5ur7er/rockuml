```rockuml tldr
@startnwdiag
nwdiag {
  internet [shape = cloud];
  internet -- firewall;

  network dmz {
    address = "203.0.113.0/24"
    firewall [address = "203.0.113.1"];
    web01 [address = "203.0.113.10"];
  }
  network internal {
    address = "10.0.0.0/24";
    web01 [address = "10.0.0.10"];
    db01 [address = "10.0.0.20", shape = database];
  }
}
@endnwdiag
```

A network diagram shows networks as horizontal bars and the servers connected to them, with their addresses. It is the classic picture of a data centre or a home lab. The syntax comes from nwdiag, a classic tool for the job, and goes between `@startnwdiag` and `@endnwdiag`, inside `nwdiag { … }`.

## Networks and servers

A `network` lists the servers on it; a server on several networks is drawn once, with a line to each. Servers take attributes in brackets: `address`, `shape`, `color` and `description`.

```rockuml
@startnwdiag
nwdiag {
  network hooli {
    address = "10.1.0.0/16"
    description = "Hooli internal"
    nucleus [address = "10.1.0.1", description = "Nucleus"];
    gavin_laptop [address = "10.1.0.66"];
  }
  network pied_piper {
    address = "192.168.0.0/24"
    description = "Pied Piper"
    nucleus [address = "192.168.0.1"];
    gilfoyle_server [address = "192.168.0.2", description = "Anton"];
    jian_yang_fridge [address = "192.168.0.3"];
  }
}
@endnwdiag
```

A server can have several addresses on one network, separated by commas: `web01 [address = "10.0.0.1, 10.0.0.2"]`.

## Network attributes

`address` labels a network, `color` fills it, `description` names it, and `width = full` stretches it across the whole diagram.

```rockuml
@startnwdiag
nwdiag {
  network backbone {
    width = full
    color = "palegreen"
    description = "Death Star backbone"
    reactor [address = "core"];
    bridge [address = "deck 1"];
  }
  network sector7 {
    color = "#FFAAAA"
    bridge;
    exhaust_port [description = "do not target", color = "orange"];
  }
}
@endnwdiag
```

## Shapes

`shape` draws a server as any element of [deployment diagrams](docs/deployment):

```rockuml
@startnwdiag
nwdiag {
  network lab {
    user [shape = actor];
    workstation [shape = node];
    files [shape = folder];
    backups [shape = storage];
    queue [shape = queue];
    db [shape = database];
    cloud_sync [shape = cloud];
  }
}
@endnwdiag
```

## Peer links and groups

`a -- b` links two servers directly, outside any network. `group` frames servers, with an optional `color` and `description`.

```rockuml
@startnwdiag
nwdiag {
  internet [shape = cloud];
  internet -- router;
  group {
    color = "#FFE4B5";
    description = "Rick's garage";
    router;
    portal_gun;
  }
  network garage {
    router;
    portal_gun;
    spaceship;
  }
}
@endnwdiag
```

## Styling

An `nwdiagDiagram` style sheet styles networks, servers, groups and links:

```rockuml
@startnwdiag
<style>
nwdiagDiagram {
  network {
    BackGroundColor LightBlue
    LineColor Navy
  }
  server {
    BackGroundColor Pink
    FontStyle bold
  }
  group {
    BackGroundColor Gold
  }
}
</style>
nwdiag {
  group {
    description = "Batcave"
    batcomputer;
  }
  network gotham {
    address = "10.0.0.0/8"
    batcomputer [address = "10.0.0.1"];
    gcpd [address = "10.0.0.2"];
  }
}
@endnwdiag
```
