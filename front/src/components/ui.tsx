import type { Component } from 'solid-js';

export const Field: Component<{ label: string; children: any }> = (props) => (
  <label class="space-y-1">
    <span class="text-gray-11 block text-xs font-medium">{props.label}</span>
    {props.children}
  </label>
);

export const Metric: Component<{ label: string; value: string }> = (props) => (
  <div class="border-gray-a5 bg-gray-a2/40 border px-3 py-2.5">
    <div class="text-gray-11 text-[11px] font-medium">{props.label}</div>
    <div class="mt-1 truncate text-lg leading-6 font-semibold tracking-[-0.005em] tabular-nums">
      {props.value}
    </div>
  </div>
);

export const Loading: Component<{ label: string }> = (props) => (
  <div class="text-gray-11 flex min-h-screen items-center justify-center">
    {props.label}
  </div>
);

export const ErrorBlock: Component<{ error: unknown }> = (props) => (
  <div class="border-red-a6 bg-red-a4 text-red-12 border px-4 py-3">
    {props.error instanceof Error ? props.error.message : 'Request failed'}
  </div>
);
