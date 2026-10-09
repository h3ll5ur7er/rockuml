import {
  ChangeDetectionStrategy,
  Component,
  inject,
  input,
  linkedSignal,
  signal,
} from '@angular/core';
import { Router } from '@angular/router';
import { encodeSource } from '@rockuml/angular';
import { RockumlPlayground } from '@rockuml/angular/editor';

/** A diagram source to edit in place, with its live diagram and buttons to copy, reset or take it elsewhere. */
@Component({
  selector: 'site-example',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [RockumlPlayground],
  templateUrl: './example.html',
  styleUrl: './example.css',
})
export class Example {
  /** The source as written; edits start from it, and Reset goes back to it. */
  readonly source = input.required<string>();
  /** Marks the TL;DR template a documentation page opens with. */
  readonly tldr = input(false);

  protected readonly edited = linkedSignal(() => this.source());
  protected readonly copied = signal(false);

  private readonly router = inject(Router);

  protected async copy(): Promise<void> {
    await navigator.clipboard.writeText(this.edited());
    this.copied.set(true);
    setTimeout(() => this.copied.set(false), 1500);
  }

  protected reset(): void {
    this.edited.set(this.source());
  }

  protected async openInPlayground(): Promise<void> {
    await this.router.navigate(['/playground'], { fragment: await encodeSource(this.edited()) });
  }
}
