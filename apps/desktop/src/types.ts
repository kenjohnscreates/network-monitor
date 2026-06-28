export interface InterfaceInfo {
  id: string;
  name: string;
  description: string | null;
  usable: boolean;
}

export type Direction = "inbound" | "outbound" | "unknown";
export type Protocol = "TCP" | "UDP" | "ICMP" | "OTHER";

export interface PacketEvent {
  timestamp: number;
  src_ip: string;
  dst_ip: string;
  src_port: number | null;
  dst_port: number | null;
  protocol: Protocol;
  packet_length: number;
  direction: Direction;
}

export interface FlowRecord {
  id: string;
  src_ip: string;
  dst_ip: string;
  hostname: string | null;
  country: string | null;
  src_port: number | null;
  dst_port: number | null;
  protocol: Protocol;
  bytes_up: number;
  bytes_down: number;
  packets_up: number;
  packets_down: number;
  first_seen: number;
  last_seen: number;
}
