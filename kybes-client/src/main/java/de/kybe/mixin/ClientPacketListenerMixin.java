package de.kybe.mixin;

import de.kybe.command.CommandManager;
import de.kybe.event.EventManager;
import de.kybe.event.events.EventLoginTail;
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

  @Inject(method = "handleLogin", at = @At("TAIL"))
  public void onHandleLogin(CallbackInfo ci) {
    EventLoginTail  event = new EventLoginTail();
    EventManager.call(event);
  }
}
