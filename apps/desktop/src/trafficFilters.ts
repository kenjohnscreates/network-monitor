/** Client-side predicates to reduce local noise for new viewers. */

import type { FlowRecord } from "./types";

/** What to emphasize in the connections table rows. */
export type FocusPreset = "all" | "internet_focus";

function isRFC1918IPv4(s: string): boolean {
  const m = /^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$/.exec(s);
  if (!m) return false;
  const a = parseInt(m[1], 10);
  const b = parseInt(m[2], 10);
  if (a === 10) return true;
  if (a === 172 && b >= 16 && b <= 31) return true;
  if (a === 192 && b === 168) return true;
  return false;
}

function isIPv4Multicast(s: string): boolean {
  const m = /^(\d+)\./.exec(s);
  return m !== null && parseInt(m[1], 10) >= 224 && parseInt(m[1], 10) <= 239;
}

function isIPv6LinkLocalOrMulticast(s: string): boolean {
  const low = s.toLowerCase().replace(/^\[+|\]+$/g, "");
  return low.startsWith("fe80:") || low.startsWith("ff");
}

function looksLikeIPv6(s: string): boolean {
  return s.includes(":");
}

function isUdpBonjourHeavy(f: FlowRecord): boolean {
  return (
    f.dst_port === 5353 ||
    f.src_port === 5353 ||
    f.dst_ip.includes("224.0.0.251") ||
    /ff02::fb/i.test(f.dst_ip) ||
    /ff02::fb/i.test(f.src_ip)
  );
}

/**
 * Rows shown when preset is `internet_focus`:
 * Drops link-local v6 , IPv4 multicast, IPv6 multicast, obvious mDNS,
 * and pure RFC1918-to-RFC1918 LAN chatter (often not what you want first).
 */
export function passesFocusPreset(f: FlowRecord, preset: FocusPreset): boolean {
  if (preset === "all") return true;

  if (
    looksLikeIPv6(f.src_ip) ||
    looksLikeIPv6(f.dst_ip) ||
    f.src_ip === "::1" ||
    f.dst_ip === "::1"
  ) {
    if (isIPv6LinkLocalOrMulticast(f.src_ip) || isIPv6LinkLocalOrMulticast(f.dst_ip))
      return false;
  }

  if (!looksLikeIPv6(f.src_ip) && !looksLikeIPv6(f.dst_ip)) {
    if (isIPv4Multicast(f.src_ip) || isIPv4Multicast(f.dst_ip)) return false;
  }

  if (isUdpBonjourHeavy(f)) return false;

  if (
    isRFC1918IPv4(f.src_ip) &&
    isRFC1918IPv4(f.dst_ip) &&
    !looksLikeIPv6(f.src_ip) &&
    !looksLikeIPv6(f.dst_ip)
  )
    return false;

  return true;
}
