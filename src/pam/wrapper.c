#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <string.h>

#include <security/pam_appl.h>


bool check_auth(char* username, char* password) {
  if (!username) {
    fprintf(stderr, "username was null");
    return false;
  }
  if (!password) {
    fprintf(stderr, "password was null");
    return false;
  }
  size_t user_len = strlen(username);
  size_t pass_len = strlen(password);
  // TODO Fill in
  return user_len && pass_len;
}
