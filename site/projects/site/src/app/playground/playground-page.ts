import { Location } from '@angular/common';
import {
  ChangeDetectionStrategy,
  Component,
  computed,
  effect,
  inject,
  signal,
} from '@angular/core';
import { ActivatedRoute } from '@angular/router';
import { RockumlRenderer, decodeSource, encodeSource, type Rendering } from 'rockuml-angular';
import { RockumlPlayground } from 'rockuml-angular/editor';

import { DIAGRAM_PAGES } from '../docs/pages';
import { loadTemplate } from '../docs/templates';

const STORAGE_KEY = 'rockuml-playground';

const STARTER = `@startuml
title The Ultimate Question
actor Arthur
participant "Deep Thought" as DT
database "Earth Mk II" as Earth

Arthur -> DT : What is the answer to life,\\nthe universe and everything?
activate DT
DT -> DT : think for 7.5 million years
DT --> Arthur : 42
deactivate DT
Arthur -> Earth : so what was the question?
activate Earth
Earth --> Arthur : demolished by Vogons\\n5 minutes before the end
deactivate Earth
note over Arthur, DT : Don't panic.
@enduml
`;

/** A full-page editor. Its source lives in the URL, so the address is a link to the diagram. */
@Component({
  selector: 'site-playground-page',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [RockumlPlayground],
  templateUrl: './playground-page.html',
  styleUrl: './playground-page.css',
})
export class PlaygroundPage {
  protected readonly source = signal(STARTER);
  protected readonly page = signal(0);
  protected readonly pageCount = signal(1);
  protected readonly pageNumbers = computed(() =>
    Array.from({ length: this.pageCount() }, (_, number) => number),
  );
  protected readonly notice = signal('');
  /** Wide diagrams shrink to fit unless shown at their actual size. */
  protected readonly actualSize = signal(false);
  protected readonly templates = DIAGRAM_PAGES;

  private readonly renderer = inject(RockumlRenderer);
  private readonly location = inject(Location);
  private svg?: string;

  constructor() {
    const fragment = inject(ActivatedRoute).snapshot.fragment;
    if (fragment) {
      decodeSource(fragment).then(
        (source) => this.source.set(source),
        () => this.say('That link holds no diagram, so here is a fresh one.'),
      );
    } else {
      this.source.set(stored() ?? STARTER);
    }

    effect((onCleanup) => {
      const source = this.source();
      const timer = setTimeout(async () => {
        this.location.replaceState(`/playground#${await encodeSource(source)}`);
        store(source);
      }, 400);
      onCleanup(() => clearTimeout(timer));
    });
  }

  protected rendered(rendering: Rendering<string>): void {
    this.svg = rendering.data;
    this.pageCount.set(rendering.pageCount);
  }

  protected async useTemplate(slug: string): Promise<void> {
    const page = this.templates.find((candidate) => candidate.slug === slug);
    if (page) {
      this.page.set(0);
      this.source.set(await loadTemplate(page));
    }
  }

  protected async share(): Promise<void> {
    const code = await encodeSource(this.source());
    await navigator.clipboard.writeText(new URL(`playground#${code}`, document.baseURI).href);
    this.say('Link copied. Anyone who opens it sees this diagram.');
  }

  protected downloadSvg(): void {
    if (this.svg) {
      download(new Blob([this.svg], { type: 'image/svg+xml' }), 'diagram.svg');
    }
  }

  protected async downloadPng(): Promise<void> {
    const { data } = await this.renderer.render(this.source(), {
      format: 'png',
      page: this.page(),
    });
    download(new Blob([data as Uint8Array<ArrayBuffer>], { type: 'image/png' }), 'diagram.png');
  }

  private say(notice: string): void {
    this.notice.set(notice);
    setTimeout(() => this.notice.set(''), 4000);
  }
}

function stored(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

function store(source: string): void {
  try {
    localStorage.setItem(STORAGE_KEY, source);
  } catch {
    // Without storage the link in the address bar is the only copy.
  }
}

function download(blob: Blob, name: string): void {
  const link = document.createElement('a');
  link.href = URL.createObjectURL(blob);
  link.download = name;
  link.click();
  URL.revokeObjectURL(link.href);
}
