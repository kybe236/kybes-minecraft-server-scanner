package de.kybe.module.modules;

import de.kybe.event.events.EventPacketSent;
import de.kybe.mixin.IMixinMovePlayerPacket;
import de.kybe.module.ToggleableModule;
import net.minecraft.network.protocol.Packet;
import net.minecraft.network.protocol.game.ServerboundMovePlayerPacket;

public class NoFallModule extends ToggleableModule {
    public NoFallModule() {
        super("NoFall");
    }
  @de.kybe.event.KybeEvents
  public void onPacketSend(EventPacketSent event) {
    if (!this.isToggled()) return;

    Packet<?> packet = event.getPacket();

    if (packet instanceof ServerboundMovePlayerPacket movePacket) {
      IMixinMovePlayerPacket mixinPacket = (IMixinMovePlayerPacket) movePacket;
      mixinPacket.setOnGround(true);
    }
  }

}
