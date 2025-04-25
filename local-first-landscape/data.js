// @ts-check
import { LandscapeSchema } from '@localfirstfm/landscape-schema'

export const data = LandscapeSchema.make({
  Version: 1,
  Id: 'y-sweet',
  Name: 'Y-Sweet',
  Website: 'https://y-sweet.dev',
  License: 'MIT',
  Deployment: ['Self-hosted', 'Hosted'],
  AppTarget: {
    Platform: {
      data: ['Browser', 'Node', 'iOS', 'Android', 'macOS', 'Linux'],
      comment: 'Limited only by Yjs support.',
    },
    LanguageSDK: {
      data: ['TypeScript', 'Python'],
    },
    FrameworkIntegrations: {
      data: ['React'],
    },
  },
  Networking: {
    Protocol: {
      data: ['WebSockets', 'HTTP'],
    },
    Topology: {
      data: 'Client-Server',
    },
  },
  ServerSideData: {
    PersistenceMechanism: {
      data: ['S3-compatible object store'],
    },
    DataModelParadigm: {
      data: 'Document',
    },
    ExistingDatabaseSupport: {
      data: 'Yes',
      comment:
        'In the "Figma architecture" sense: ground-truth document data only lives on S3, but document metadata exists in your existing DB.',
    },
  },
  ClientSideData: {
    LocalRefreshLatency: {
      data: '<1 ms',
    },
    PersistenceMechanism: {
      data: ['IndexedDB'],
    },
    DataModel: {
      data: 'Document',
    },
    SchemaManagement: {
      data: ['No support'],
    },
    OfflineReads: {
      data: 'Yes',
    },
    OptimisticUpdates: {
      data: 'Yes',
    },
    OfflineWrites: {
      data: 'Yes',
    },
  },
  SynchronizationStrategy: {
    ConflictHandling: {
      data: 'Automatic via CRDT',
      comment: 'CRDT (YATA)',
    },
    WhereResolutionOccurs: {
      data: 'Server',
    },
    WhatGetsSynced: {
      data: {
        ClientToServer: 'mutations',
      },
    },
    Authority: {
      data: 'Centralized',
    },
  },
  AuthIdentity: {
    Encryption: {
      data: 'Yes',
      comment:
        'Subject to the underlying object store used; most encrypt at rest by default; AES-GCM encryption on the client',
    },
    AuthenticationMethod: {
      data: ['Tokens'],
      comment:
        'Auth delegation from your application server based on temporary tokens.',
    },
  },
  UIRelated: {
    Components: {
      data: ['y.js plugins', 'Presence'],
      comment:
        'Anything compatible with Yjs (Slate, BlockNote, Quill, Lexical, etc.)',
    },
  },
})
