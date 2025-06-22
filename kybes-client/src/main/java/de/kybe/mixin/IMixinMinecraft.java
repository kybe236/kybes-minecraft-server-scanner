package de.kybe.mixin;

import com.mojang.authlib.minecraft.UserApiService;
import com.mojang.authlib.yggdrasil.ProfileResult;
import com.mojang.authlib.yggdrasil.YggdrasilAuthenticationService;
import net.minecraft.client.Minecraft;
import net.minecraft.client.User;
import net.minecraft.client.gui.screens.social.PlayerSocialManager;
import net.minecraft.client.multiplayer.ProfileKeyPairManager;
import net.minecraft.client.multiplayer.chat.report.ReportingContext;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Mutable;
import org.spongepowered.asm.mixin.gen.Accessor;

import java.util.concurrent.CompletableFuture;

@Mixin(Minecraft.class)
public interface IMixinMinecraft {
  @Accessor("user")
  @Mutable
  void setUser(User user);

  @Accessor("authenticationService")
  YggdrasilAuthenticationService getAuthenticationService();

  @Accessor("userApiService")
  @Mutable
  void setUserApiService(UserApiService userApiService);

  @Accessor("playerSocialManager")
  @Mutable
  void setPlayerSocialManager(PlayerSocialManager socialInteractionsManager);

  @Accessor("profileKeyPairManager")
  @Mutable
  void setProfileKeyPairManager(ProfileKeyPairManager profileKeyPairManager);

  @Accessor("reportingContext")
  @Mutable
  void setReportingContext(ReportingContext abuseReportContext);

  @Accessor("profileFuture")
  @Mutable
  void setProfileFuture(CompletableFuture<ProfileResult> gameProfileFuture);
}