import { createMiddleware } from "@solidjs/start/middleware";

export default createMiddleware([
  async (event, next) => {
    console.log("Request received:", event.req.url);
    const startedAt = Date.now();

    const response = await next();

    console.log(`Request took ${Date.now() - startedAt}ms`);
    return response;
  },
]);
