import { supportedUrl } from "./capture-policy.js";
export function excluded(policy, value) {
  if (!policy || !supportedUrl(value)) return true;
  const url = new URL(value);
  url.hash = "";
  const host = url.hostname.toLowerCase();
  return (
    policy.excluded_sites.some((pattern) =>
      pattern.startsWith("*.")
        ? host.endsWith(`.${pattern.slice(2)}`) &&
          host.length > pattern.length - 2
        : host === pattern,
    ) ||
    policy.excluded_addresses.some((pattern) =>
      pattern.endsWith("*")
        ? url.href.startsWith(pattern.slice(0, -1))
        : url.href === pattern,
    )
  );
}
export function allowed(policy, browser, item, forced) {
  if (
    !policy?.browsers.includes(browser) ||
    excluded(policy, item.url) ||
    excluded(policy, item.finalUrl || item.url)
  )
    return false;
  const name = (
    item.filename ||
    decodeURIComponent(new URL(item.finalUrl || item.url).pathname)
  )
    .split(/[\\/]/u)
    .pop();
  const extension = name.includes(".")
    ? name.split(".").pop().toLowerCase()
    : "";
  return forced || policy.extensions.includes(extension);
}
