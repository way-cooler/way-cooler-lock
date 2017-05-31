#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <string.h>

#include <security/pam_appl.h>


static int pam_conv_handler(int num_msg,
                            const struct pam_message **msg,
                            struct pam_response **resp,
                            void *appdata_ptr) {
  // Validate num_msg
  if (num_msg > PAM_MAX_NUM_MSG) {
    return PAM_CONV_ERR;
  }

  // Allocate empty responses for each message
  struct pam_response *responses;
  if (!(responses = calloc(num_msg, sizeof(struct pam_response)))) {
    // If the allocation failed, return PAM_BUF_ERR
    return PAM_BUF_ERR;
  }
  for (int i = 0; i < num_msg; i++) {
    // Ignore all PAM messages except those prompting for hidden input
    if (msg[i]->msg_style != PAM_PROMPT_ECHO_OFF) {
      continue;
    }
    size_t len = strlen(appdata_ptr) + 1;
    if (!(responses[i].resp = malloc(len))) {
      goto error;
    }
    strncpy(responses[i].resp, appdata_ptr, len);
  }
  *resp = responses;
  return PAM_SUCCESS;

 error:
  for (int i = 0; i < num_msg; i++) {
    if (responses[i].resp) {
      free(responses[i].resp);
    }
  }
  free(responses);
  return PAM_BUF_ERR;
}


bool check_auth(char* username, char* password) {
  if (!username) {
    fprintf(stderr, "username was null");
    return false;
  }
  if (!password) {
    fprintf(stderr, "password was null");
    return false;
  }
  size_t pass_len = strlen(password);
  if (pass_len > PAM_MAX_MSG_SIZE - 1 || pass_len < 1) {
    return false;
  }

  struct pam_conv conv = { &pam_conv_handler, (void *) password };
  pam_handle_t *handle;
  pam_start("wc-lock", username, &conv, &handle);
  int result = pam_authenticate(handle,
                                PAM_SILENT | PAM_DISALLOW_NULL_AUTHTOK);
  pam_end(handle, result);

  return (result == PAM_SUCCESS);
}
