// The documentation's pages, in reading order. Each page's Markdown is its own chunk, loaded when visited.

export interface DocPage {
  slug: string;
  title: string;
  /** The line under the title, also shown on the page's cards. */
  summary: string;
  section: string;
  load: () => Promise<string>;
}

type Entry = Omit<DocPage, 'section'>;

const text = (load: () => Promise<{ default: string }>) => () =>
  load().then((module) => module.default);

const SECTIONS: { title: string; pages: Entry[] }[] = [
  {
    title: 'Start here',
    pages: [
      {
        slug: 'getting-started',
        title: 'Getting started',
        summary: 'Install rockuml, draw your first diagram and find your way around.',
        load: text(() => import('./content/getting-started.md')),
      },
      {
        slug: 'basics',
        title: 'Common commands',
        summary:
          'Titles, notes, comments, legends, scaling and pages: what every diagram understands.',
        load: text(() => import('./content/basics.md')),
      },
    ],
  },
  {
    title: 'Diagrams',
    pages: [
      {
        slug: 'sequence',
        title: 'Sequence diagrams',
        summary: 'Who says what to whom, and in which order.',
        load: text(() => import('./content/sequence.md')),
      },
      {
        slug: 'use-case',
        title: 'Use case diagrams',
        summary: 'Actors, what they want from a system, and where the system ends.',
        load: text(() => import('./content/use-case.md')),
      },
      {
        slug: 'class',
        title: 'Class diagrams',
        summary: 'Classes, interfaces and enums, their members and how they relate.',
        load: text(() => import('./content/class.md')),
      },
      {
        slug: 'object',
        title: 'Object diagrams',
        summary: 'Objects, maps and their links: a snapshot of a running system.',
        load: text(() => import('./content/object.md')),
      },
      {
        slug: 'activity',
        title: 'Activity diagrams',
        summary: 'Flows with decisions, loops, parallel branches and swimlanes.',
        load: text(() => import('./content/activity.md')),
      },
      {
        slug: 'component',
        title: 'Component diagrams',
        summary: 'Building blocks, the interfaces they offer and what they depend on.',
        load: text(() => import('./content/component.md')),
      },
      {
        slug: 'deployment',
        title: 'Deployment diagrams',
        summary: 'Nodes, clouds, databases, queues: everything that runs your software.',
        load: text(() => import('./content/deployment.md')),
      },
      {
        slug: 'state',
        title: 'State diagrams',
        summary: 'States, transitions, composite and concurrent states.',
        load: text(() => import('./content/state.md')),
      },
      {
        slug: 'timing',
        title: 'Timing diagrams',
        summary: 'Signals, states and messages along a time axis.',
        load: text(() => import('./content/timing.md')),
      },
      {
        slug: 'archimate',
        title: 'ArchiMate diagrams',
        summary: 'Enterprise architecture with ArchiMate elements and relations.',
        load: text(() => import('./content/archimate.md')),
      },
      {
        slug: 'chen',
        title: 'Entity relationships',
        summary: 'Entities, relationships and attributes in Chen notation.',
        load: text(() => import('./content/chen.md')),
      },
      {
        slug: 'mindmap',
        title: 'Mind maps',
        summary: 'Ideas branching out from a central topic.',
        load: text(() => import('./content/mindmap.md')),
      },
      {
        slug: 'wbs',
        title: 'Work breakdown structures',
        summary: 'A project broken down into its deliverables.',
        load: text(() => import('./content/wbs.md')),
      },
      {
        slug: 'gantt',
        title: 'Gantt charts',
        summary: 'Tasks, milestones and resources on a calendar, written as plain sentences.',
        load: text(() => import('./content/gantt.md')),
      },
      {
        slug: 'json',
        title: 'JSON data',
        summary: 'JSON documents drawn as nested tables, with highlights.',
        load: text(() => import('./content/json.md')),
      },
      {
        slug: 'yaml',
        title: 'YAML data',
        summary: 'YAML documents drawn as nested tables, with highlights.',
        load: text(() => import('./content/yaml.md')),
      },
      {
        slug: 'network',
        title: 'Network diagrams',
        summary: 'Networks, the servers on them and their addresses.',
        load: text(() => import('./content/network.md')),
      },
      {
        slug: 'salt',
        title: 'Wireframes',
        summary: 'Sketch user interfaces in text with Salt.',
        load: text(() => import('./content/salt.md')),
      },
    ],
  },
  {
    title: 'Text and style',
    pages: [
      {
        slug: 'creole',
        title: 'Text formatting',
        summary: 'Creole markup for any label: emphasis, lists, tables, colours and fonts.',
        load: text(() => import('./content/creole.md')),
      },
      {
        slug: 'styling',
        title: 'Colours and styles',
        summary: 'Inline colours, skinparam and style sheets.',
        load: text(() => import('./content/styling.md')),
      },
      {
        slug: 'themes',
        title: 'Themes',
        summary: 'Restyle a whole diagram with one line.',
        load: text(() => import('./content/themes.md')),
      },
      {
        slug: 'sprites',
        title: 'Sprites, images and icons',
        summary: 'Pictures inside diagrams: sprites, images, Open Iconic and emoji.',
        load: text(() => import('./content/sprites.md')),
      },
      {
        slug: 'preprocessor',
        title: 'Preprocessor',
        summary: 'Variables, functions, conditions, loops and includes.',
        load: text(() => import('./content/preprocessor.md')),
      },
    ],
  },
  {
    title: 'Tools',
    pages: [
      {
        slug: 'cli',
        title: 'Command line',
        summary: 'The rockuml binary: files, formats, pipes, and every flag.',
        load: text(() => import('./content/cli.md')),
      },
      {
        slug: 'server',
        title: 'Server for editors',
        summary: 'Run rockuml as the PlantUML server of your editor plugin.',
        load: text(() => import('./content/server.md')),
      },
      {
        slug: 'javascript',
        title: 'JavaScript and WebAssembly',
        summary: 'Render diagrams in browsers and Node with rockuml.wasm.',
        load: text(() => import('./content/javascript.md')),
      },
      {
        slug: 'angular',
        title: 'Angular components',
        summary: 'Live diagrams and editors for Angular applications.',
        load: text(() => import('./content/angular.md')),
      },
      {
        slug: 'compatibility',
        title: 'Compatibility',
        summary: 'How rockuml relates to PlantUML, and what it does not do (yet).',
        load: text(() => import('./content/compatibility.md')),
      },
      {
        slug: 'features',
        title: 'Feature status',
        summary: 'Every PlantUML feature, and whether rockuml ports it, plans to or never will.',
        load: text(() => import('./content/features.md')),
      },
    ],
  },
];

export const DOC_SECTIONS: { title: string; pages: DocPage[] }[] = SECTIONS.map((section) => ({
  title: section.title,
  pages: section.pages.map((page) => ({ ...page, section: section.title })),
}));

export const DOC_PAGES: DocPage[] = DOC_SECTIONS.flatMap((section) => section.pages);

export const DIAGRAM_PAGES: DocPage[] = DOC_SECTIONS.find(
  (section) => section.title === 'Diagrams',
)!.pages;

export function findDocPage(slug: string): DocPage | undefined {
  return DOC_PAGES.find((page) => page.slug === slug);
}
