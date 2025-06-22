package de.kybe.event.events;

import net.minecraft.network.protocol.Packet;

public class EventPacketReceive extends CancelableEvent {
  private Packet<?> packet;

  public EventPacketReceive(Packet<?> packet) {
    this.packet = packet;
  }

  public Packet<?> getPacket() {
    return packet;
  }

  public void setPacket(Packet<?> packet) {
    this.packet = packet;
  }
}