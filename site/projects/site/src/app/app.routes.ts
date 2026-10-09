import { Routes } from '@angular/router';

export const routes: Routes = [
  {
    path: '',
    title: 'rockuml — PlantUML diagrams, rendered by Rust',
    loadComponent: () => import('./home/home').then((module) => module.Home),
  },
  {
    path: 'playground',
    title: 'Playground · rockuml',
    loadComponent: () =>
      import('./playground/playground-page').then((module) => module.PlaygroundPage),
  },
  {
    path: 'docs',
    loadComponent: () => import('./docs/docs-shell').then((module) => module.DocsShell),
    children: [
      { path: '', pathMatch: 'full', redirectTo: 'getting-started' },
      {
        path: ':slug',
        loadComponent: () => import('./docs/doc-page').then((module) => module.DocPageView),
      },
    ],
  },
  {
    path: '**',
    title: 'Not found · rockuml',
    loadComponent: () => import('./not-found').then((module) => module.NotFound),
  },
];
