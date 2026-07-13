// ══════════════════════════════════════════════════════════════
// jc9 Notes — TypeScript type definitions
// ══════════════════════════════════════════════════════════════

export interface NoteGroup {
  id: string
  name: string
  parentId: string | null
  sortOrder: number
  createdAt: string
  updatedAt: string
}

export interface Note {
  id: string
  groupId: string | null
  title: string
  content: string
  format: 'plain' | 'markdown'
  isPinned: boolean
  tags: string[]
  visibility: 'PRIVATE' | 'PUBLIC' | 'PROTECTED'
  sortOrder: number
  version: number
  isDeleted: boolean
  isArchived: boolean
  createdAt: string
  updatedAt: string
}

// ── Note version history ──

export interface NoteVersion {
  id: string
  noteId: string
  title: string
  content: string
  format: string
  tags: string[]
  version: number
  createdAt: string
}

// ── Editor state ──

export interface EditorState {
  title: string
  content: string
  format: 'plain' | 'markdown'
  groupId: string | null
  tags: string[]
  visibility: 'PRIVATE' | 'PUBLIC' | 'PROTECTED'
}
