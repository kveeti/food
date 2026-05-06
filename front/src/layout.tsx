import { A } from '@solidjs/router';
import type { Component } from 'solid-js';

export const Shell: Component<{ children: any }> = (props) => (
  <main class="min-h-screen bg-gray-1 text-gray-12">{props.children}</main>
);

export const Page: Component<{ children: any }> = (props) => (
  <div class="mx-auto max-w-160 px-3 pt-16 pb-34 sm:pt-14">
    {props.children}
  </div>
);

export const Nav: Component = () => (
  <nav class="bg-gray-1 border-gray-a3 fixed right-0 bottom-0 left-0 z-100 w-full border-t sm:top-0 sm:bottom-[unset] sm:border-t-0 sm:border-b">
    <div class="pwa:px-8 pwa:pb-16 mx-auto flex h-11 sm:h-8 w-full max-w-160 justify-center sm:justify-start sm:px-3">
      <ul class="flex items-stretch select-none">
        <li>
          <NavLink href="/">today</NavLink>
        </li>
        <li>
          <NavLink href="/settings">settings</NavLink>
        </li>
      </ul>
    </div>
  </nav>
);

const NavLink: Component<{
  href: string;
  children: any;
}> = (props) => (
  <A
    activeClass="bg-gray-a3"
    class="focus inline-flex h-full items-center px-3 text-xs font-medium whitespace-nowrap"
    end
    href={props.href}
  >
    {props.children}
  </A>
);
