package de.kybe.utils;

import com.mojang.authlib.minecraft.UserApiService;
import de.kybe.mixin.IMixinMinecraft;
import net.minecraft.Util;
import net.minecraft.client.User;
import net.minecraft.client.gui.screens.social.PlayerSocialManager;
import net.minecraft.client.multiplayer.ProfileKeyPairManager;
import net.minecraft.client.multiplayer.chat.report.ReportEnvironment;
import net.minecraft.client.multiplayer.chat.report.ReportingContext;

import java.nio.charset.StandardCharsets;
import java.util.Optional;
import java.util.UUID;
import java.util.concurrent.CompletableFuture;

import static de.kybe.Constants.mc;

public class CrackedUtils {
  public static void login(String username) {
    User user = new User(username, UUID.nameUUIDFromBytes(("OfflinePlayer:" + username).getBytes(StandardCharsets.UTF_8)), "", Optional.empty(), Optional.empty(), User.Type.MOJANG);
    setSession(user);
  }

  public static void setSession(User session) {
    IMixinMinecraft mca = (IMixinMinecraft) mc;
    mca.setUser(session);
    UserApiService apiService;
    apiService = mca.getAuthenticationService().createUserApiService(session.getAccessToken());
    mca.setUserApiService(apiService);
    mca.setPlayerSocialManager(new PlayerSocialManager(mc, apiService));
    mca.setProfileKeyPairManager(ProfileKeyPairManager.create(apiService, session, mc.gameDirectory.toPath()));
    mca.setReportingContext(ReportingContext.create(ReportEnvironment.local(), apiService));
    mca.setProfileFuture(CompletableFuture.supplyAsync(() -> mc.getMinecraftSessionService().fetchProfile(mc.getUser().getProfileId(), true), Util.ioPool()));
  }
}
