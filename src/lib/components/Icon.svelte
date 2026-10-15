<script lang="ts">
  // Keel uses Lucide icons, drawn at 16px with a 1.75 stroke in currentColor.
  // GitOut refers to them by these short names.
  import type { Component } from "svelte";
  import ArchiveRestore from "@lucide/svelte/icons/archive-restore";
  import Archive from "@lucide/svelte/icons/archive";
  import ArrowDownToLine from "@lucide/svelte/icons/arrow-down-to-line";
  import ArrowLeft from "@lucide/svelte/icons/arrow-left";
  import ArrowRight from "@lucide/svelte/icons/arrow-right";
  import ArrowUpFromLine from "@lucide/svelte/icons/arrow-up-from-line";
  import Check from "@lucide/svelte/icons/check";
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import ChevronUp from "@lucide/svelte/icons/chevron-up";
  import CircleCheck from "@lucide/svelte/icons/circle-check";
  import CircleX from "@lucide/svelte/icons/circle-x";
  import CloudDownload from "@lucide/svelte/icons/cloud-download";
  import Cloud from "@lucide/svelte/icons/cloud";
  import Copy from "@lucide/svelte/icons/copy";
  import Download from "@lucide/svelte/icons/download";
  import Ellipsis from "@lucide/svelte/icons/ellipsis";
  import ExternalLink from "@lucide/svelte/icons/external-link";
  import File from "@lucide/svelte/icons/file";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import FolderPlus from "@lucide/svelte/icons/folder-plus";
  import Folder from "@lucide/svelte/icons/folder";
  import GitBranch from "@lucide/svelte/icons/git-branch";
  import GitCommitHorizontal from "@lucide/svelte/icons/git-commit-horizontal";
  import GitMerge from "@lucide/svelte/icons/git-merge";
  import House from "@lucide/svelte/icons/house";
  import Info from "@lucide/svelte/icons/info";
  import Minus from "@lucide/svelte/icons/minus";
  import Package from "@lucide/svelte/icons/package";
  import Palette from "@lucide/svelte/icons/palette";
  import Pencil from "@lucide/svelte/icons/pencil";
  import Plus from "@lucide/svelte/icons/plus";
  import RotateCw from "@lucide/svelte/icons/rotate-cw";
  import Search from "@lucide/svelte/icons/search";
  import Settings from "@lucide/svelte/icons/settings";
  import SlidersHorizontal from "@lucide/svelte/icons/sliders-horizontal";
  import Tag from "@lucide/svelte/icons/tag";
  import Terminal from "@lucide/svelte/icons/terminal";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import Undo2 from "@lucide/svelte/icons/undo-2";
  import UserRound from "@lucide/svelte/icons/user-round";
  import X from "@lucide/svelte/icons/x";

  const ICONS: Record<string, Component<any>> = {
    pull: ArrowDownToLine,
    push: ArrowUpFromLine,
    fetch: CloudDownload,
    refresh: RotateCw,
    branch: GitBranch,
    stash: Archive,
    pop: ArchiveRestore,
    settings: Settings,
    folder: Folder,
    "folder-open": FolderOpen,
    "folder-plus": FolderPlus,
    plus: Plus,
    minus: Minus,
    x: X,
    check: Check,
    chevron: ChevronRight,
    down: ChevronDown,
    up: ChevronUp,
    forward: ArrowRight,
    back: ArrowLeft,
    tag: Tag,
    cloud: Cloud,
    user: UserRound,
    search: Search,
    undo: Undo2,
    more: Ellipsis,
    commit: GitCommitHorizontal,
    merge: GitMerge,
    release: Package,
    pencil: Pencil,
    file: File,
    external: ExternalLink,
    terminal: Terminal,
    copy: Copy,
    warning: TriangleAlert,
    success: CircleCheck,
    error: CircleX,
    info: Info,
    home: House,
    download: Download,
    palette: Palette,
    sliders: SlidersHorizontal,
  };

  let { name, size = 16, stroke = 1.75 }: { name: string; size?: number; stroke?: number } = $props();

  let Glyph = $derived(ICONS[name]);
</script>

{#if Glyph}
  <Glyph {size} strokeWidth={stroke} aria-hidden="true" class="k-icon" style="width:{size}px;height:{size}px;stroke-width:{stroke}" />
{/if}
