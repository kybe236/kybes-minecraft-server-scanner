package de.kybe.mixin;

import net.minecraft.client.gui.screens.multiplayer.JoinMultiplayerScreen;
import net.minecraft.client.gui.screens.multiplayer.ServerSelectionList;
import net.minecraft.client.multiplayer.ServerData;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(ServerSelectionList.OnlineServerEntry.class)
public class OnlineServerEntryMixin {
  @Inject(method = "<init>", at = @At("TAIL"))
  private void onInit(ServerSelectionList serverSelectionList, JoinMultiplayerScreen joinMultiplayerScreen, ServerData serverData, CallbackInfo ci) {

  }
}
