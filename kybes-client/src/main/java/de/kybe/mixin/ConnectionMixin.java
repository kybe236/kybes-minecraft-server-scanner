package de.kybe.mixin;

import de.kybe.event.EventManager;
import de.kybe.event.events.EventPacketReceive;
import de.kybe.event.events.EventPacketSent;
import io.netty.channel.ChannelFutureListener;
import net.minecraft.network.Connection;
import net.minecraft.network.PacketListener;
import net.minecraft.network.protocol.Packet;
import org.jetbrains.annotations.Nullable;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(Connection.class)
public class ConnectionMixin {
  @Inject(method = "send(Lnet/minecraft/network/protocol/Packet;Lio/netty/channel/ChannelFutureListener;Z)V",
    at = @At("HEAD"),
    cancellable = true)
  private void onSend(Packet<?> packet, @Nullable ChannelFutureListener listener, boolean bl, CallbackInfo ci) {
    EventPacketSent event = new EventPacketSent(packet);
    EventManager.call(event);

    if (event.isCancelled()) {
      ci.cancel();
    } else if (event.getPacket() != packet) {
      ci.cancel();

      Connection connection = (Connection)(Object)this;
      connection.send(event.getPacket(), listener, bl);
    }
  }

  @SuppressWarnings("unchecked")
  @Inject(method = "genericsFtw",
    at = @At("HEAD"),
    cancellable = true)
  private static void onGenericsFtw(Packet<?> packet, PacketListener listener, CallbackInfo ci) {
    EventPacketReceive event = new EventPacketReceive(packet);
    EventManager.call(event);

    if (event.isCancelled()) {
      ci.cancel(); // cancel handling packet
    } else if (event.getPacket() != packet) {
      ci.cancel();

      ((Packet) event.getPacket()).handle((PacketListener) listener);
    }
  }
}
