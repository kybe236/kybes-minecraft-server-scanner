package de.kybe.mixin;

import de.kybe.command.CommandManager;
import net.minecraft.client.multiplayer.ClientPacketListener;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(ClientPacketListener.class)
public class ClientPacketListenerMixin {
  @Inject(method = "sendChat", at = @At("HEAD"), cancellable = true)
  public void onSendChat(String input, CallbackInfo ci) {
    if (!input.startsWith("+")) return;
    ci.cancel();
    CommandManager.handleInput(input);
  }
}
