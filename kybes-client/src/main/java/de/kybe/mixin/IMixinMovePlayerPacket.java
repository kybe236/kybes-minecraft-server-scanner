package de.kybe.mixin;

import net.minecraft.network.protocol.game.ServerboundMovePlayerPacket;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Mutable;
import org.spongepowered.asm.mixin.gen.Accessor;

@Mixin(ServerboundMovePlayerPacket.class)
public interface IMixinMovePlayerPacket {
  @Accessor("onGround")
  @Mutable
  void setOnGround(boolean onGround);
}