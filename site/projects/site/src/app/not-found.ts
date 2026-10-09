import { ChangeDetectionStrategy, Component } from '@angular/core';
import { RouterLink } from '@angular/router';

@Component({
  selector: 'site-not-found',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [RouterLink],
  styles: `
    :host {
      display: block;
      max-width: 640px;
      margin: 80px auto;
      padding: 0 var(--gutter);
      text-align: center;
    }
  `,
  template: `
    <h1>404: this page has been eaten by a Grue</h1>
    <p>
      It is pitch dark here. You might want to head back to the <a routerLink="/">home page</a> or
      the <a routerLink="/docs">documentation</a>.
    </p>
  `,
})
export class NotFound {}
