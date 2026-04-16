import { writable } from 'svelte/store';
import type { SettingsSection, SidebarMode } from './types';

export const sidebarMode = writable<SidebarMode>('tables');
export const activeSettingsSection = writable<SettingsSection>('updates');
