#import <Foundation/Foundation.h>
#import <Security/Security.h>
#include <stdlib.h>
#include <string.h>

static NSMutableDictionary *isotope_generic_password_query(NSString *service,
                                                           NSString *account) {
  NSMutableDictionary *query = [@{
    (__bridge id)kSecClass: (__bridge id)kSecClassGenericPassword,
    (__bridge id)kSecAttrService: service,
    (__bridge id)kSecAttrAccount: account,
  } mutableCopy];

  SecKeychainRef defaultKeychain = NULL;
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
  OSStatus status = SecKeychainCopyDefault(&defaultKeychain);
#pragma clang diagnostic pop
  if (status == errSecSuccess && defaultKeychain != NULL) {
    query[(__bridge id)kSecUseKeychain] = CFBridgingRelease(defaultKeychain);
  }

  return query;
}

char *isotope_copy_generic_password_json_with_status(const char *service_cstr,
                                                     const char *account_cstr,
                                                     char **error_cstr,
                                                     int *status_out);
bool isotope_generic_password_exists(const char *service_cstr,
                                     const char *account_cstr,
                                     char **error_cstr,
                                     int *status_out);

char *isotope_copy_generic_password_json(const char *service_cstr,
                                         const char *account_cstr,
                                         char **error_cstr) {
  return isotope_copy_generic_password_json_with_status(service_cstr,
                                                        account_cstr,
                                                        error_cstr, NULL);
}

char *isotope_copy_generic_password_json_with_status(const char *service_cstr,
                                                     const char *account_cstr,
                                                     char **error_cstr,
                                                     int *status_out) {
  @autoreleasepool {
    if (error_cstr != NULL) {
      *error_cstr = NULL;
    }
    if (status_out != NULL) {
      *status_out = errSecSuccess;
    }

    if (service_cstr == NULL || account_cstr == NULL) {
      if (error_cstr != NULL) {
        *error_cstr = strdup("invalid keychain lookup arguments");
      }
      return NULL;
    }

    NSString *service = [NSString stringWithUTF8String:service_cstr];
    NSString *account = [NSString stringWithUTF8String:account_cstr];
    if (service == nil || account == nil) {
      if (error_cstr != NULL) {
        *error_cstr = strdup("keychain lookup arguments must be UTF-8");
      }
      return NULL;
    }

    NSMutableDictionary *query = isotope_generic_password_query(service, account);
    query[(__bridge id)kSecReturnData] = @YES;
    query[(__bridge id)kSecMatchLimit] = (__bridge id)kSecMatchLimitOne;

    CFTypeRef result = NULL;
    OSStatus status = SecItemCopyMatching((__bridge CFDictionaryRef)query, &result);
    if (status_out != NULL) {
      *status_out = (int)status;
    }
    if (status != errSecSuccess) {
      if (error_cstr != NULL) {
        NSString *message = (__bridge_transfer NSString *)
            SecCopyErrorMessageString(status, NULL);
        if (message == nil) {
          message = [NSString stringWithFormat:@"keychain lookup failed (%d)",
                                                (int)status];
        }
        *error_cstr = strdup(message.UTF8String);
      }
      return NULL;
    }

    NSData *data = CFBridgingRelease(result);
    if (data == nil) {
      if (error_cstr != NULL) {
        *error_cstr = strdup("keychain lookup did not return data");
      }
      return NULL;
    }

    char *copy = calloc(data.length + 1, sizeof(char));
    if (copy == NULL) {
      if (error_cstr != NULL) {
        *error_cstr = strdup("failed to allocate keychain buffer");
      }
      return NULL;
    }
    memcpy(copy, data.bytes, data.length);
    copy[data.length] = '\0';
    return copy;
  }
}

bool isotope_generic_password_exists(const char *service_cstr,
                                     const char *account_cstr,
                                     char **error_cstr,
                                     int *status_out) {
  @autoreleasepool {
    if (error_cstr != NULL) {
      *error_cstr = NULL;
    }
    if (status_out != NULL) {
      *status_out = errSecSuccess;
    }

    if (service_cstr == NULL || account_cstr == NULL) {
      if (error_cstr != NULL) {
        *error_cstr = strdup("invalid keychain lookup arguments");
      }
      return false;
    }

    NSString *service = [NSString stringWithUTF8String:service_cstr];
    NSString *account = [NSString stringWithUTF8String:account_cstr];
    if (service == nil || account == nil) {
      if (error_cstr != NULL) {
        *error_cstr = strdup("keychain lookup arguments must be UTF-8");
      }
      return false;
    }

    NSMutableDictionary *query = isotope_generic_password_query(service, account);
    query[(__bridge id)kSecMatchLimit] = (__bridge id)kSecMatchLimitOne;

    OSStatus status = SecItemCopyMatching((__bridge CFDictionaryRef)query, NULL);
    if (status_out != NULL) {
      *status_out = (int)status;
    }
    if (status == errSecSuccess) {
      return true;
    }

    if (error_cstr != NULL) {
      NSString *message = (__bridge_transfer NSString *)
          SecCopyErrorMessageString(status, NULL);
      if (message == nil) {
        message = [NSString stringWithFormat:@"keychain lookup failed (%d)",
                                              (int)status];
      }
      *error_cstr = strdup(message.UTF8String);
    }
    return false;
  }
}

void isotope_free_c_string(char *value) {
  if (value != NULL) {
    free(value);
  }
}

bool isotope_post_distributed_notification_with_object(const char *name_cstr,
                                                       const char *object_cstr,
                                                       char **error_cstr);

bool isotope_post_distributed_notification(const char *name_cstr,
                                           char **error_cstr) {
  return isotope_post_distributed_notification_with_object(name_cstr, NULL,
                                                          error_cstr);
}

bool isotope_post_distributed_notification_with_object(const char *name_cstr,
                                                       const char *object_cstr,
                                                       char **error_cstr) {
  @autoreleasepool {
    if (error_cstr != NULL) {
      *error_cstr = NULL;
    }

    if (name_cstr == NULL) {
      if (error_cstr != NULL) {
        *error_cstr = strdup("invalid distributed notification name");
      }
      return false;
    }

    NSString *name = [NSString stringWithUTF8String:name_cstr];
    if (name.length == 0) {
      if (error_cstr != NULL) {
        *error_cstr = strdup("distributed notification name must be UTF-8");
      }
      return false;
    }

    NSString *object = nil;
    if (object_cstr != NULL) {
      object = [NSString stringWithUTF8String:object_cstr];
      if (object.length == 0) {
        if (error_cstr != NULL) {
          *error_cstr = strdup("distributed notification object must be UTF-8");
        }
        return false;
      }
    }

    [[NSDistributedNotificationCenter defaultCenter]
        postNotificationName:name
                      object:object
                    userInfo:nil
          deliverImmediately:YES];
    return true;
  }
}

bool isotope_store_generic_password_json(const char *service_cstr,
                                         const char *account_cstr,
                                         const char *value_cstr,
                                         char **error_cstr) {
  @autoreleasepool {
    if (error_cstr != NULL) {
      *error_cstr = NULL;
    }

    if (service_cstr == NULL || account_cstr == NULL || value_cstr == NULL) {
      if (error_cstr != NULL) {
        *error_cstr = strdup("invalid keychain write arguments");
      }
      return false;
    }

    NSString *service = [NSString stringWithUTF8String:service_cstr];
    NSString *account = [NSString stringWithUTF8String:account_cstr];
    NSString *value = [NSString stringWithUTF8String:value_cstr];
    if (service == nil || account == nil || value == nil) {
      if (error_cstr != NULL) {
        *error_cstr = strdup("keychain write arguments must be UTF-8");
      }
      return false;
    }

    NSData *data = [value dataUsingEncoding:NSUTF8StringEncoding];
    NSMutableDictionary *query = isotope_generic_password_query(service, account);

    NSDictionary *attributes = @{(__bridge id)kSecValueData: data};
    OSStatus status =
        SecItemUpdate((__bridge CFDictionaryRef)query,
                      (__bridge CFDictionaryRef)attributes);
    if (status == errSecItemNotFound) {
      NSMutableDictionary *createQuery = [query mutableCopy];
      createQuery[(__bridge id)kSecValueData] = data;
      status = SecItemAdd((__bridge CFDictionaryRef)createQuery, NULL);
    }

    if (status != errSecSuccess) {
      if (error_cstr != NULL) {
        NSString *message = (__bridge_transfer NSString *)
            SecCopyErrorMessageString(status, NULL);
        if (message == nil) {
          message = [NSString stringWithFormat:@"keychain write failed (%d)",
                                                (int)status];
        }
        *error_cstr = strdup(message.UTF8String);
      }
      return false;
    }

    return true;
  }
}
