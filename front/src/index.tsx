/* @refresh reload */
import './styles.css';
import { QueryClient, QueryClientProvider } from '@tanstack/solid-query';
import { render } from 'solid-js/web';
import 'solid-devtools';

import App from './App';

const root = document.getElementById('root');

if (import.meta.env.DEV && !(root instanceof HTMLElement)) {
  throw new Error(
    'Root element not found. Did you forget to add it to your index.html? Or maybe the id attribute got misspelled?',
  );
}

const queryClient = new QueryClient();

render(() => (
  <QueryClientProvider client={queryClient}>
    <App />
  </QueryClientProvider>
), root!);
