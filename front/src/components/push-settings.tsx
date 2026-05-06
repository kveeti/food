import { useMutation, useQuery, useQueryClient } from '@tanstack/solid-query';
import { Show, type Component } from 'solid-js';

import {
  api,
  browserPushStateQueryOptions,
  pushPublicKeyQueryOptions,
} from '../app.data';
import { urlBase64ToUint8Array } from '../lib/push';

export const PushSettings: Component<{ settingsSet: boolean }> = (props) => {
  const queryClient = useQueryClient();
  const publicKey = useQuery(() => pushPublicKeyQueryOptions());
  const browserState = useQuery(() => browserPushStateQueryOptions());

  const invalidateBrowserPushState = () => {
    queryClient.invalidateQueries({ queryKey: ['push', 'browser-state'] });
  };

  const enablePush = useMutation(() => ({
    mutationFn: async () => {
      const key = publicKey.data;
      if (!key?.configured || !key.publicKey) {
        throw new Error('push_not_configured');
      }
      if (!('Notification' in window)) {
        throw new Error('push_unsupported');
      }
      const permission =
        Notification.permission === 'granted'
          ? 'granted'
          : await Notification.requestPermission();

      if (permission !== 'granted') {
        throw new Error('notification_permission_denied');
      }

      const registration = await navigator.serviceWorker.register('/sw.js');
      const subscription =
        (await registration.pushManager.getSubscription()) ??
        (await registration.pushManager.subscribe({
          userVisibleOnly: true,
          applicationServerKey: urlBase64ToUint8Array(key.publicKey),
        }));
      const json = subscription.toJSON();

      return api<{ ok: boolean }>({
        path: '/api/v1/push/subscribe',
        method: 'POST',
        body: {
          endpoint: subscription.endpoint,
          p256dh: json.keys?.p256dh,
          auth: json.keys?.auth,
        },
      });
    },
    onSuccess: invalidateBrowserPushState,
  }));

  const disablePush = useMutation(() => ({
    mutationFn: async () => {
      const registration = await navigator.serviceWorker.ready;
      const subscription = await registration.pushManager.getSubscription();
      if (subscription) {
        await api<{ ok: boolean }>({
          path: '/api/v1/push/unsubscribe',
          method: 'POST',
          body: { endpoint: subscription.endpoint },
        });
        await subscription.unsubscribe();
      }
    },
    onSuccess: invalidateBrowserPushState,
  }));

  const testPush = useMutation(() => ({
    mutationFn: () =>
      api<{ ok: boolean }>({
        path: '/api/v1/push/test',
        method: 'POST',
      }),
  }));

  const busy = () =>
    enablePush.isPending || disablePush.isPending || testPush.isPending;

  const pushConfigured = () => Boolean(publicKey.data?.configured);
  const subscribed = () => Boolean(browserState.data?.subscribed);
  const canEnable = () => pushConfigured() && !subscribed();
  const canTest = () => pushConfigured() && subscribed();
  const pushError = () => enablePush.error ?? disablePush.error ?? testPush.error;
  const enableIfReady = () => {
    if (busy() || !canEnable()) return;
    enablePush.mutate();
  };
  const disableIfReady = () => {
    if (busy() || !subscribed()) return;
    disablePush.mutate();
  };
  const testIfReady = () => {
    if (busy() || !canTest()) return;
    testPush.mutate();
  };

  return (
    <section class="space-y-4">
      <div class="flex flex-col justify-between gap-3 sm:flex-row sm:items-center">
        <div>
          <h2 class="text-lg leading-6 font-semibold tracking-[-0.005em]">
            Notifications
          </h2>
          <p class="text-gray-11 mt-1 text-xs">
            {browserState.data?.subscribed ? 'Enabled' : 'Disabled'}
          </p>
        </div>
        <div class="flex flex-wrap gap-2">
          <Show when={canEnable()}>
            <button
              type="button"
              class="button-secondary"
              onClick={enableIfReady}
            >
              Enable
            </button>
          </Show>
          <Show when={subscribed()}>
            <button
              type="button"
              class="button-secondary"
              onClick={disableIfReady}
            >
              Disable
            </button>
          </Show>
          <Show when={canTest()}>
            <button
              type="button"
              class="button-secondary"
              onClick={testIfReady}
            >
              Test
            </button>
          </Show>
        </div>
      </div>

      <Show when={!publicKey.isLoading && !publicKey.data?.configured}>
        <p class="border-gray-a6 bg-gray-a2 border px-3 py-2 text-sm">
          push_not_configured
        </p>
      </Show>
      <Show when={!props.settingsSet}>
        <p class="border-gray-a6 bg-gray-a2 border px-3 py-2 text-sm">
          settings_required
        </p>
      </Show>
      <Show when={pushError()}>
        {(error) => (
          <p class="border-red-a6 bg-red-a4 text-red-12 border px-3 py-2 text-sm">
            {error().message}
          </p>
        )}
      </Show>
    </section>
  );
};
