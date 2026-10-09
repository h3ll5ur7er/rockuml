import { ChangeDetectionStrategy, Component, resource } from '@angular/core';
import { RouterLink } from '@angular/router';
import { RockumlDiagram } from '@rockuml/angular';

import { DIAGRAM_PAGES } from '../docs/pages';
import { loadTemplate } from '../docs/templates';
import { Example } from '../shared/example';
import { RELEASES } from '../shared/links';

const HERO = `@startuml
title Pied Piper, the middle-out edition
actor Richard
participant "Compression\\nengine" as Engine
database "The Box" as Box

Richard -> Engine : compress(everything)
activate Engine
Engine -> Engine : middle-out
Engine --> Richard : Weissman score 5.2
deactivate Engine
Richard -> Box : ship it
note right of Box : Gavin approves.\\nNobody else does.
@enduml`;

@Component({
  selector: 'site-home',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [RouterLink, Example, RockumlDiagram],
  templateUrl: './home.html',
  styleUrl: './home.css',
})
export class Home {
  protected readonly hero = HERO;
  protected readonly releases = RELEASES;
  protected readonly gallery = resource({
    loader: () =>
      Promise.all(DIAGRAM_PAGES.map(async (page) => ({ page, source: await loadTemplate(page) }))),
  });
}
